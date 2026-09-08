//! Parallel, incremental, cancellable indexer.
//!
//! ```text
//!  roots ──► crawler (walkdir, exclude globs, hidden filter)
//!              │ changed/new files (fingerprint ≠ manifest)
//!              ▼
//!        work channel ──► N extraction workers ──► IndexWriter (shared)
//!                                   │ fingerprints
//!                                   ▼
//!                           manifest batch writer
//!  after crawl: manifest paths not seen → delete from index + manifest
//!  commit → reader reload
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel as chan;
use globset::{Glob, GlobSet, GlobSetBuilder};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tantivy::schema::document::TantivyDocument;
use tantivy::{IndexWriter, Term};
use walkdir::WalkDir;

use super::manifest::{Fingerprint, Manifest};
use super::schema::Fields;
use crate::config::Settings;
use crate::error::{Error, Result};
use crate::extract::{self, classify, ExtractOptions};
use crate::fs_util::{extension_of, glob_form, is_hidden, path_key, to_unix_secs};

/// Phase of the indexing job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndexPhase {
    #[default]
    Idle,
    Scanning,
    Extracting,
    Cleaning,
    Committing,
    Done,
    Cancelled,
    Failed,
}

/// Live counters, safe to share across threads.
#[derive(Debug, Default)]
pub struct Counters {
    pub scanned: AtomicU64,
    pub queued: AtomicU64,
    pub indexed: AtomicU64,
    pub unchanged: AtomicU64,
    pub name_only: AtomicU64,
    pub failed: AtomicU64,
    pub deleted: AtomicU64,
    pub bytes: AtomicU64,
    pub current: Mutex<Option<String>>,
    pub phase: Mutex<IndexPhase>,
    pub error: Mutex<Option<String>>,
}

/// Snapshot of [`Counters`] for the UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    pub phase: IndexPhase,
    pub scanned: u64,
    pub queued: u64,
    pub indexed: u64,
    pub unchanged: u64,
    pub name_only: u64,
    pub failed: u64,
    pub deleted: u64,
    pub bytes: u64,
    pub current: Option<String>,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

impl Counters {
    pub fn snapshot(&self, started: Instant) -> IndexSummary {
        IndexSummary {
            phase: *self.phase.lock(),
            scanned: self.scanned.load(Ordering::Relaxed),
            queued: self.queued.load(Ordering::Relaxed),
            indexed: self.indexed.load(Ordering::Relaxed),
            unchanged: self.unchanged.load(Ordering::Relaxed),
            name_only: self.name_only.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            deleted: self.deleted.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            current: self.current.lock().clone(),
            error: self.error.lock().clone(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }

    fn set_phase(&self, p: IndexPhase) {
        *self.phase.lock() = p;
    }
}

/// Receives progress notifications (the desktop app forwards them as events).
pub trait ProgressSink: Send + Sync {
    fn on_progress(&self, summary: &IndexSummary);
}

impl<F: Fn(&IndexSummary) + Send + Sync> ProgressSink for F {
    fn on_progress(&self, summary: &IndexSummary) {
        self(summary)
    }
}

/// One indexing run.
pub struct IndexJob {
    pub settings: Settings,
    pub writer: Arc<RwLock<IndexWriter>>,
    pub fields: Fields,
    pub manifest: Arc<Manifest>,
    pub counters: Arc<Counters>,
    pub cancel: Arc<AtomicBool>,
    pub sink: Option<Arc<dyn ProgressSink>>,
    pub started: Instant,
    /// Ignore fingerprints and re-extract everything.
    pub full: bool,
}

pub(crate) struct WorkItem {
    path: PathBuf,
    key: String,
    root: String,
    ext: String,
    size: u64,
    modified: i64,
}

pub(crate) fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let mut pat = p.replace('\\', "/");
        // Bare folder names ("node_modules") mean "anywhere".
        if !pat.contains('/') && !pat.contains('*') {
            pat = format!("**/{pat}/**");
        } else if !pat.starts_with("**") && !pat.starts_with('/') && !pat.contains(":/") {
            pat = format!("**/{pat}");
        }
        let g = Glob::new(&pat)
            .map_err(|e| Error::Config(format!("bad exclude pattern `{p}`: {e}")))?;
        b.add(g);
    }
    b.build().map_err(|e| Error::Config(e.to_string()))
}

impl IndexJob {
    fn notify(&self) {
        if let Some(s) = &self.sink {
            s.on_progress(&self.counters.snapshot(self.started));
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Run the job to completion (or cancellation). Always commits whatever
    /// was indexed so far, so progress is never lost.
    pub fn run(self) -> Result<IndexSummary> {
        let result = self.run_inner();
        let summary = self.counters.snapshot(self.started);
        match &result {
            Ok(()) => {}
            Err(Error::Cancelled) => self.counters.set_phase(IndexPhase::Cancelled),
            Err(e) => {
                *self.counters.error.lock() = Some(e.to_string());
                self.counters.set_phase(IndexPhase::Failed);
            }
        }
        self.notify();
        result.map(|_| summary)
    }

    fn run_inner(&self) -> Result<()> {
        let excludes = build_globset(&self.settings.exclude_globs)?;
        let opts = ExtractOptions {
            max_file_size: self.settings.max_file_size_bytes,
            extract_pdf: self.settings.extract_pdf,
            ..ExtractOptions::default()
        };
        let workers = self.settings.effective_workers();
        let (work_tx, work_rx) = chan::bounded::<WorkItem>(workers * 8);
        let (fp_tx, fp_rx) = chan::unbounded::<(String, Fingerprint)>();

        self.counters.set_phase(IndexPhase::Scanning);
        self.notify();

        // --- extraction workers ---------------------------------------------
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let rx = work_rx.clone();
            let fp_tx = fp_tx.clone();
            let writer = self.writer.clone();
            let fields = self.fields;
            let counters = self.counters.clone();
            let cancel = self.cancel.clone();
            let store_content = self.settings.store_content;
            handles.push(
                std::thread::Builder::new()
                    .name("qidir-extract".into())
                    .spawn(move || {
                        while let Ok(item) = rx.recv() {
                            if cancel.load(Ordering::Relaxed) {
                                // Drain quickly.
                                continue;
                            }
                            *counters.current.lock() = Some(item.key.clone());
                            let fp =
                                index_one(&writer, &fields, &item, &opts, store_content, &counters);
                            let _ = fp_tx.send((item.key, fp));
                        }
                    })?,
            );
        }
        drop(work_rx);
        drop(fp_tx);

        // --- manifest batch writer -------------------------------------------
        let manifest = self.manifest.clone();
        let seen: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let seen_w = seen.clone();
        let manifest_thread = std::thread::Builder::new()
            .name("qidir-manifest".into())
            .spawn(move || -> Result<()> {
                let mut batch: Vec<(String, Fingerprint)> = Vec::with_capacity(512);
                loop {
                    match fp_rx.recv_timeout(std::time::Duration::from_millis(500)) {
                        Ok(item) => {
                            seen_w.lock().insert(item.0.clone());
                            batch.push(item);
                            if batch.len() >= 512 {
                                manifest.put_many(batch.iter().map(|(k, v)| (k.as_str(), *v)))?;
                                batch.clear();
                            }
                        }
                        Err(chan::RecvTimeoutError::Timeout) => {
                            if !batch.is_empty() {
                                manifest.put_many(batch.iter().map(|(k, v)| (k.as_str(), *v)))?;
                                batch.clear();
                            }
                        }
                        Err(chan::RecvTimeoutError::Disconnected) => break,
                    }
                }
                if !batch.is_empty() {
                    manifest.put_many(batch.iter().map(|(k, v)| (k.as_str(), *v)))?;
                }
                Ok(())
            })?;

        // --- crawl -------------------------------------------------------------
        let mut last_notify = Instant::now();
        let crawl_result = (|| -> Result<()> {
            for root in self.settings.roots.iter().filter(|r| r.enabled) {
                let root_key = path_key(&root.path);
                if !root.path.exists() {
                    tracing::warn!(root = %root_key, "root does not exist, skipping");
                    continue;
                }
                let walker = WalkDir::new(&root.path)
                    .follow_links(self.settings.follow_links)
                    .same_file_system(false)
                    .into_iter()
                    .filter_entry(|e| {
                        let p = e.path();
                        if excludes.is_match(glob_form(p)) {
                            return false;
                        }
                        if !self.settings.index_hidden {
                            if let Ok(meta) = e.metadata() {
                                if p != root.path && is_hidden(p, &meta) {
                                    return false;
                                }
                            }
                        }
                        true
                    });
                for entry in walker {
                    if self.cancelled() {
                        return Err(Error::Cancelled);
                    }
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::debug!(error = %e, "walk error");
                            continue;
                        }
                    };
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    let meta = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    self.counters.scanned.fetch_add(1, Ordering::Relaxed);
                    let path = entry.path();
                    let ext = extension_of(path);
                    if !self.settings.extension_allowed(&ext) {
                        continue;
                    }
                    let key = path_key(path);
                    let size = meta.len();
                    let modified = meta.modified().map(to_unix_secs).unwrap_or(0);
                    seen.lock().insert(key.clone());

                    if !self.full {
                        if let Ok(Some(fp)) = self.manifest.get(&key) {
                            if fp.modified == modified && fp.size == size {
                                self.counters.unchanged.fetch_add(1, Ordering::Relaxed);
                                continue;
                            }
                        }
                    }
                    self.counters.queued.fetch_add(1, Ordering::Relaxed);
                    work_tx
                        .send(WorkItem {
                            path: path.to_path_buf(),
                            key,
                            root: root_key.clone(),
                            ext,
                            size,
                            modified,
                        })
                        .map_err(|_| Error::Other("worker channel closed".into()))?;

                    if last_notify.elapsed().as_millis() >= 250 {
                        self.notify();
                        last_notify = Instant::now();
                    }
                }
            }
            Ok(())
        })();
        drop(work_tx);

        self.counters.set_phase(IndexPhase::Extracting);
        self.notify();
        for h in handles {
            let _ = h.join();
        }
        manifest_thread
            .join()
            .map_err(|_| Error::Other("manifest thread panicked".into()))??;

        if let Err(Error::Cancelled) = crawl_result {
            self.commit()?;
            return Err(Error::Cancelled);
        }
        crawl_result?;
        if self.cancelled() {
            self.commit()?;
            return Err(Error::Cancelled);
        }

        // --- deletions ----------------------------------------------------------
        self.counters.set_phase(IndexPhase::Cleaning);
        *self.counters.current.lock() = None;
        self.notify();
        let enabled_roots: Vec<String> = self
            .settings
            .roots
            .iter()
            .filter(|r| r.enabled)
            .map(|r| path_key(&r.path))
            .collect();
        let seen = seen.lock();
        let mut stale = Vec::new();
        for known in self.manifest.all_paths()? {
            let under_enabled_root = enabled_roots.iter().any(|r| is_under(&known, r));
            if !under_enabled_root || !seen.contains(&known) {
                stale.push(known);
            }
        }
        drop(seen);
        {
            let w = self.writer.read();
            for key in &stale {
                w.delete_term(Term::from_field_text(self.fields.path, key));
            }
        }
        if !stale.is_empty() {
            self.manifest
                .remove_many(stale.iter().map(|s| s.as_str()))?;
            self.counters
                .deleted
                .fetch_add(stale.len() as u64, Ordering::Relaxed);
        }

        self.commit()?;
        self.counters.set_phase(IndexPhase::Done);
        self.manifest
            .set_meta("last_indexed", &chrono::Utc::now().timestamp().to_string())?;
        Ok(())
    }

    fn commit(&self) -> Result<()> {
        self.counters.set_phase(IndexPhase::Committing);
        self.notify();
        commit_writer(&self.writer)
    }
}

/// Commit through the shared writer: takes the write lock, so it waits for
/// in-flight `add_document` calls (which hold read locks) to finish.
pub(crate) fn commit_writer(writer: &RwLock<IndexWriter>) -> Result<()> {
    writer.write().commit()?;
    Ok(())
}

/// Windows paths are case-insensitive; compare accordingly.
pub(crate) fn is_under(path: &str, root: &str) -> bool {
    let root_norm = root.trim_end_matches(['\\', '/']);
    if path.len() < root_norm.len() {
        return false;
    }
    let (head, tail) = path.split_at(root_norm.len());
    let same = if cfg!(windows) {
        head.eq_ignore_ascii_case(root_norm)
    } else {
        head == root_norm
    };
    same && (tail.is_empty() || tail.starts_with('\\') || tail.starts_with('/'))
}

/// Extract + write one file. Returns the fingerprint to record.
pub(crate) fn index_one(
    writer: &RwLock<IndexWriter>,
    fields: &Fields,
    item: &WorkItem,
    opts: &ExtractOptions,
    store_content: bool,
    counters: &Counters,
) -> Fingerprint {
    let (category, _) = classify(&item.ext);
    let mut doc = TantivyDocument::new();
    doc.add_text(fields.path, &item.key);
    doc.add_text(fields.path_text, parent_for_text(&item.key));
    doc.add_text(
        fields.name,
        item.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    doc.add_text(fields.ext, &item.ext);
    doc.add_text(fields.category, category.as_str());
    doc.add_text(fields.root, &item.root);
    doc.add_u64(fields.size, item.size);
    doc.add_i64(fields.modified, item.modified);
    doc.add_i64(fields.indexed_at, chrono::Utc::now().timestamp());

    let mut has_content = false;
    match extract::extract_file(&item.path, &item.ext, item.size, opts) {
        Ok(Some(extracted)) => {
            has_content = !extracted.text.is_empty();
            counters.bytes.fetch_add(item.size, Ordering::Relaxed);
            doc.add_text(fields.content, &extracted.text);
            doc.add_text(fields.content_exact, &extracted.text);
            if store_content {
                doc.add_text(fields.body, &extracted.text);
            }
            if let Some(enc) = extracted.encoding {
                doc.add_text(fields.encoding, enc);
            }
        }
        Ok(None) => {
            counters.name_only.fetch_add(1, Ordering::Relaxed);
        }
        Err(e) => {
            tracing::debug!(path = %item.key, error = %e, "extraction failed; indexing by name");
            counters.failed.fetch_add(1, Ordering::Relaxed);
        }
    }
    doc.add_u64(fields.has_content, has_content as u64);

    let w = writer.read();
    w.delete_term(Term::from_field_text(fields.path, &item.key));
    if let Err(e) = w.add_document(doc) {
        tracing::error!(path = %item.key, error = %e, "add_document failed");
        counters.failed.fetch_add(1, Ordering::Relaxed);
    } else {
        counters.indexed.fetch_add(1, Ordering::Relaxed);
    }
    Fingerprint {
        modified: item.modified,
        size: item.size,
        has_content,
    }
}

/// Folder part of a path for `path_text` indexing.
fn parent_for_text(key: &str) -> &str {
    match key.rfind(['\\', '/']) {
        Some(i) => &key[..i],
        None => "",
    }
}

/// Index or remove a single path (used by the watcher).
pub(crate) fn update_single(
    writer: &RwLock<IndexWriter>,
    fields: &Fields,
    manifest: &Manifest,
    settings: &Settings,
    path: &Path,
    counters: &Counters,
) -> Result<bool> {
    let key = path_key(path);
    let root = settings
        .roots
        .iter()
        .filter(|r| r.enabled)
        .map(|r| path_key(&r.path))
        .find(|r| is_under(&key, r));
    let Some(root) = root else {
        return Ok(false);
    };
    let excludes = build_globset(&settings.exclude_globs)?;
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => {
            if excludes.is_match(glob_form(path))
                || (!settings.index_hidden && is_hidden(path, &meta))
            {
                return Ok(false);
            }
            let ext = extension_of(path);
            if !settings.extension_allowed(&ext) {
                return Ok(false);
            }
            let item = WorkItem {
                path: path.to_path_buf(),
                key: key.clone(),
                root,
                ext,
                size: meta.len(),
                modified: meta.modified().map(to_unix_secs).unwrap_or(0),
            };
            if let Ok(Some(fp)) = manifest.get(&key) {
                if fp.modified == item.modified && fp.size == item.size {
                    return Ok(false);
                }
            }
            let opts = ExtractOptions {
                max_file_size: settings.max_file_size_bytes,
                extract_pdf: settings.extract_pdf,
                ..ExtractOptions::default()
            };
            let fp = index_one(
                writer,
                fields,
                &item,
                &opts,
                settings.store_content,
                counters,
            );
            manifest.put_many([(key.as_str(), fp)])?;
            Ok(true)
        }
        Ok(_) => Ok(false), // directory: handled by the caller via a rescan
        Err(_) => {
            // Removed: delete the file itself or everything under a folder.
            let mut victims = vec![key.clone()];
            let sep = if key.contains('\\') { '\\' } else { '/' };
            victims.extend(manifest.paths_with_prefix(&format!("{key}{sep}"))?);
            let mut any = false;
            let w = writer.read();
            for v in &victims {
                if manifest.get(v)?.is_some() {
                    w.delete_term(Term::from_field_text(fields.path, v));
                    any = true;
                }
            }
            if any {
                manifest.remove_many(victims.iter().map(|s| s.as_str()))?;
                counters
                    .deleted
                    .fetch_add(victims.len() as u64, Ordering::Relaxed);
            }
            Ok(any)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_root() {
        assert!(is_under(r"C:\Docs\a.txt", r"C:\Docs"));
        assert!(is_under(r"C:\Docs\a.txt", r"C:\Docs\"));
        assert!(!is_under(r"C:\Documents\a.txt", r"C:\Docs"));
        assert!(is_under("/home/u/f", "/home/u"));
        assert!(!is_under("/home/user/f", "/home/u"));
    }

    #[test]
    fn globset_forms() {
        let gs = build_globset(&[
            "node_modules".into(),
            "*.tmp".into(),
            "**/.git/**".into(),
            "C:/Temp/**".into(),
        ])
        .unwrap();
        assert!(gs.is_match("C:/x/node_modules/a.js"));
        assert!(gs.is_match("C:/x/y/z.tmp"));
        assert!(gs.is_match("D:/repo/.git/HEAD"));
        assert!(gs.is_match("C:/Temp/a.txt"));
        assert!(!gs.is_match("C:/x/y/z.txt"));
    }
}
