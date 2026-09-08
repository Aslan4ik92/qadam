//! [`Engine`]: the thread-safe facade used by the desktop app and the CLI.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel as chan;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tantivy::collector::Count;
use tantivy::query::{AllQuery, TermQuery};
use tantivy::schema::document::TantivyDocument;
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::analysis::AnalysisMode;
use crate::config::{AppPaths, Settings};
use crate::error::{Error, Result};
use crate::extract::{self, ExtractOptions};
use crate::fs_util::{extension_of, path_key};
use crate::index::indexer::{self, Counters, IndexPhase, IndexSummary, ProgressSink};
use crate::index::watcher::Watcher;
use crate::index::{open_or_create_index, schema::SCHEMA_VERSION, Fields, IndexJob, Manifest};
use crate::search::{
    self, highlight, Matcher, QueryBuilder, SearchMode, SearchRequest, SearchResponse,
};

/// Progress report exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub running: bool,
    #[serde(flatten)]
    pub summary: IndexSummary,
}

/// Coarse engine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    Idle,
    Indexing,
}

/// Aggregate statistics about the index.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub documents: u64,
    pub with_content: u64,
    pub index_size_bytes: u64,
    /// Unix seconds of the last completed indexing run.
    pub last_indexed: Option<i64>,
    pub roots: Vec<RootStats>,
    pub indexing: bool,
    pub watching: bool,
    pub schema_version: u32,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RootStats {
    pub path: String,
    pub enabled: bool,
    pub exists: bool,
    pub documents: u64,
}

/// A segment of preview text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSegment {
    pub text: String,
    pub highlight: bool,
}

/// Document preview with highlighted matches.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: String,
    pub segments: Vec<PreviewSegment>,
    pub total_matches: usize,
    pub truncated: bool,
    /// `index` when served from the stored body, `file` when re-extracted.
    pub source: String,
    pub encoding: Option<String>,
    pub chars: usize,
}

/// Maximum characters returned in a preview.
pub const PREVIEW_MAX_CHARS: usize = 1_500_000;

struct ProgressRelay {
    engine_sink: Arc<RwLock<Option<Arc<dyn ProgressSink>>>>,
    last: Arc<Mutex<IndexSummary>>,
}

impl ProgressSink for ProgressRelay {
    fn on_progress(&self, summary: &IndexSummary) {
        *self.last.lock() = summary.clone();
        if let Some(s) = self.engine_sink.read().as_ref() {
            s.on_progress(summary);
        }
    }
}

pub struct Engine {
    paths: AppPaths,
    settings: RwLock<Settings>,
    index: Index,
    fields: Fields,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    manifest: Arc<Manifest>,
    /// Serializes full indexing runs and incremental updates.
    job_lock: Arc<Mutex<()>>,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    counters: RwLock<Arc<Counters>>,
    started: RwLock<Instant>,
    last_summary: Arc<Mutex<IndexSummary>>,
    sink: Arc<RwLock<Option<Arc<dyn ProgressSink>>>>,
    watcher: Mutex<Option<Watcher>>,
}

impl Engine {
    /// Open (or create) the engine at the default per-user location.
    pub fn open_default() -> Result<Arc<Self>> {
        Self::open(AppPaths::default_paths()?)
    }

    /// Open (or create) the engine under `paths`.
    pub fn open(paths: AppPaths) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&paths.data_dir).map_err(|e| Error::io(&paths.data_dir, e))?;
        let settings = Settings::load(&paths.settings_file)?;
        Self::open_with_settings(paths, settings)
    }

    pub fn open_with_settings(paths: AppPaths, settings: Settings) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&paths.data_dir).map_err(|e| Error::io(&paths.data_dir, e))?;
        let manifest_path = paths.data_dir.join("manifest.redb");
        let mut manifest = Manifest::open(&manifest_path)?;

        // Schema migration: rebuild from scratch when the layout changed.
        let stored_version = manifest
            .get_meta("schema_version")?
            .and_then(|v| v.parse::<u32>().ok());
        if stored_version != Some(SCHEMA_VERSION) && paths.index_dir.exists() {
            tracing::info!(
                ?stored_version,
                SCHEMA_VERSION,
                "schema changed, rebuilding index"
            );
            drop(manifest);
            let _ = std::fs::remove_dir_all(&paths.index_dir);
            let _ = std::fs::remove_file(&manifest_path);
            manifest = Manifest::open(&manifest_path)?;
        }
        manifest.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;

        let (index, fields) = open_or_create_index(&paths.index_dir)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = Self::create_writer(&index, &settings)?;

        Ok(Arc::new(Self {
            paths,
            settings: RwLock::new(settings),
            index,
            fields,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            manifest: Arc::new(manifest),
            job_lock: Arc::new(Mutex::new(())),
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            counters: RwLock::new(Arc::new(Counters::default())),
            started: RwLock::new(Instant::now()),
            last_summary: Arc::new(Mutex::new(IndexSummary::default())),
            sink: Arc::new(RwLock::new(None)),
            watcher: Mutex::new(None),
        }))
    }

    fn create_writer(index: &Index, settings: &Settings) -> Result<IndexWriter> {
        let threads = settings.effective_workers().clamp(1, 8);
        let per_thread_min = 16 * 1024 * 1024;
        let budget = (settings.writer_memory_mb * 1024 * 1024).max(threads * per_thread_min);
        match index.writer_with_num_threads(threads, budget) {
            Ok(w) => Ok(w),
            Err(tantivy::TantivyError::LockFailure(..)) => Err(Error::IndexLocked),
            Err(e) => Err(e.into()),
        }
    }

    // ------------------------------------------------------------------ settings

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    /// Replace settings, persist them and restart the watcher if needed.
    pub fn update_settings(self: &Arc<Self>, mut new: Settings) -> Result<Settings> {
        new.normalize();
        new.save(&self.paths.settings_file)?;
        let watching = self.watcher.lock().is_some();
        *self.settings.write() = new.clone();
        if watching || new.watch_changes {
            self.stop_watching();
            if new.watch_changes {
                self.start_watching()?;
            }
        }
        Ok(new)
    }

    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    // ------------------------------------------------------------------ indexing

    /// Register a sink for progress events (replaces the previous one).
    pub fn set_progress_sink(&self, sink: Arc<dyn ProgressSink>) {
        *self.sink.write() = Some(sink);
    }

    pub fn state(&self) -> IndexState {
        if self.running.load(Ordering::SeqCst) {
            IndexState::Indexing
        } else {
            IndexState::Idle
        }
    }

    /// Current progress (live counters while running, last summary otherwise).
    pub fn progress(&self) -> IndexProgress {
        let running = self.running.load(Ordering::SeqCst);
        let summary = if running {
            self.counters.read().snapshot(*self.started.read())
        } else {
            self.last_summary.lock().clone()
        };
        IndexProgress { running, summary }
    }

    /// Start indexing in a background thread. `full` re-extracts every file.
    pub fn start_indexing(self: &Arc<Self>, full: bool) -> Result<()> {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Error::AlreadyIndexing);
        }
        self.cancel.store(false, Ordering::SeqCst);
        let counters = Arc::new(Counters::default());
        *self.counters.write() = counters.clone();
        *self.started.write() = Instant::now();
        let started = *self.started.read();

        let job = IndexJob {
            settings: self.settings(),
            writer: self.writer.clone(),
            fields: self.fields,
            manifest: self.manifest.clone(),
            counters,
            cancel: self.cancel.clone(),
            sink: Some(Arc::new(ProgressRelay {
                engine_sink: self.sink.clone(),
                last: self.last_summary.clone(),
            })),
            started,
            full,
        };
        let engine = Arc::clone(self);
        std::thread::Builder::new()
            .name("qidir-index".into())
            .spawn(move || {
                let _guard = engine.job_lock.lock();
                let result = job.run();
                match &result {
                    Ok(s) => tracing::info!(
                        indexed = s.indexed,
                        deleted = s.deleted,
                        unchanged = s.unchanged,
                        "indexing finished"
                    ),
                    Err(Error::Cancelled) => tracing::info!("indexing cancelled"),
                    Err(e) => tracing::error!(error = %e, "indexing failed"),
                }
                if let Err(e) = engine.reader.reload() {
                    tracing::error!(error = %e, "reader reload failed");
                }
                engine.running.store(false, Ordering::SeqCst);
            })?;
        Ok(())
    }

    /// Run indexing synchronously on the calling thread (CLI).
    pub fn index_blocking(self: &Arc<Self>, full: bool) -> Result<IndexSummary> {
        self.start_indexing(full)?;
        while self.running.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let summary = self.last_summary.lock().clone();
        match summary.phase {
            IndexPhase::Cancelled => Err(Error::Cancelled),
            IndexPhase::Failed => Err(Error::Other(summary.error.clone().unwrap_or_default())),
            _ => Ok(summary),
        }
    }

    pub fn cancel_indexing(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Wait until a running job finishes (used on shutdown).
    pub fn wait_idle(&self) {
        while self.running.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Index/remove specific paths (watcher & drag-drop). Directories are
    /// scanned recursively.
    pub fn update_paths(&self, paths: &[PathBuf]) -> Result<usize> {
        let _guard = self.job_lock.lock();
        let settings = self.settings();
        let counters = Counters::default();
        let mut changed = 0usize;
        let excludes = indexer::build_globset(&settings.exclude_globs)?;
        for p in paths {
            if excludes.is_match(crate::fs_util::glob_form(p)) {
                continue;
            }
            if p.is_dir() {
                for entry in walkdir::WalkDir::new(p)
                    .follow_links(settings.follow_links)
                    .into_iter()
                    .filter_entry(|e| !excludes.is_match(crate::fs_util::glob_form(e.path())))
                    .flatten()
                {
                    if entry.file_type().is_file()
                        && indexer::update_single(
                            &self.writer,
                            &self.fields,
                            &self.manifest,
                            &settings,
                            entry.path(),
                            &counters,
                        )?
                    {
                        changed += 1;
                    }
                }
            } else if indexer::update_single(
                &self.writer,
                &self.fields,
                &self.manifest,
                &settings,
                p,
                &counters,
            )? {
                changed += 1;
            }
        }
        if changed > 0 {
            indexer::commit_writer(&self.writer)?;
            self.reader.reload()?;
        }
        Ok(changed)
    }

    /// Remove every document and fingerprint.
    pub fn clear_index(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(Error::AlreadyIndexing);
        }
        let _guard = self.job_lock.lock();
        {
            let mut w = self.writer.write();
            w.delete_all_documents()?;
            w.commit()?;
        }
        self.manifest.clear()?;
        self.reader.reload()?;
        *self.last_summary.lock() = IndexSummary::default();
        Ok(())
    }

    // ------------------------------------------------------------------ watching

    /// Watch enabled roots for changes. No-op if already watching.
    pub fn start_watching(self: &Arc<Self>) -> Result<()> {
        let mut slot = self.watcher.lock();
        if slot.is_some() {
            return Ok(());
        }
        let roots: Vec<PathBuf> = self
            .settings
            .read()
            .roots
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.path.clone())
            .collect();
        if roots.is_empty() {
            return Ok(());
        }
        let (tx, rx) = chan::unbounded::<Vec<PathBuf>>();
        let watcher = Watcher::start(&roots, tx)?;
        let engine = Arc::clone(self);
        std::thread::Builder::new()
            .name("qidir-watch".into())
            .spawn(move || {
                while let Ok(mut paths) = rx.recv() {
                    // Coalesce bursts.
                    while let Ok(more) = rx.try_recv() {
                        paths.extend(more);
                    }
                    paths.sort();
                    paths.dedup();
                    match engine.update_paths(&paths) {
                        Ok(n) if n > 0 => tracing::info!(changed = n, "watcher applied updates"),
                        Ok(_) => {}
                        Err(e) => tracing::warn!(error = %e, "watcher update failed"),
                    }
                }
            })?;
        *slot = Some(watcher);
        Ok(())
    }

    pub fn stop_watching(&self) {
        *self.watcher.lock() = None;
    }

    pub fn is_watching(&self) -> bool {
        self.watcher.lock().is_some()
    }

    // ------------------------------------------------------------------ search

    pub fn search(&self, req: &SearchRequest) -> Result<SearchResponse> {
        let searcher = self.reader.searcher();
        search::search(&searcher, &self.fields, req)
    }

    /// Full text of a document with the query's matches highlighted.
    pub fn preview(&self, path: &str, query: &str, mode: SearchMode) -> Result<Preview> {
        let searcher = self.reader.searcher();
        let mut text: Option<String> = None;
        let mut encoding: Option<String> = None;
        let mut source = "file";

        // 1. Stored body.
        let term = Term::from_field_text(self.fields.path, path);
        let q = TermQuery::new(term, IndexRecordOption::Basic);
        let top = searcher.search(&q, &tantivy::collector::TopDocs::with_limit(1))?;
        if let Some((_, addr)) = top.first() {
            let doc: TantivyDocument = searcher.doc(*addr)?;
            if let Some(body) = doc.get_first(self.fields.body).and_then(|v| v.as_str()) {
                text = Some(body.to_string());
                source = "index";
            }
            encoding = doc
                .get_first(self.fields.encoding)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        // 2. Re-extract from disk.
        if text.is_none() {
            let p = Path::new(path);
            let meta = std::fs::metadata(p).map_err(|e| Error::io(p, e))?;
            let settings = self.settings();
            let opts = ExtractOptions {
                max_file_size: settings.max_file_size_bytes.max(meta.len()),
                extract_pdf: true,
                ..ExtractOptions::default()
            };
            match extract::extract_file(p, &extension_of(p), meta.len(), &opts)? {
                Some(e) => {
                    encoding = e.encoding.map(|s| s.to_string());
                    text = Some(e.text);
                }
                None => return Err(Error::Unsupported(extension_of(p))),
            }
        }
        let full = text.unwrap_or_default();
        let (body, truncated) = truncate_chars(&full, PREVIEW_MAX_CHARS);

        let builder = QueryBuilder::new(self.fields, mode);
        let parsed = builder.parse(query);
        let matcher: Matcher = builder.matcher(&parsed);
        let ranges = highlight::find_matches(body, &matcher, mode.analysis(), 0);
        let segments = segments_from_ranges(body, &ranges);
        Ok(Preview {
            path: path.to_string(),
            total_matches: ranges.len(),
            segments,
            truncated,
            source: source.to_string(),
            encoding,
            chars: body.chars().count(),
        })
    }

    /// Analyze arbitrary text (diagnostics / "how will this be searched?").
    pub fn analyze_text(&self, text: &str, mode: SearchMode) -> Vec<String> {
        crate::analysis::analyze(text, mode.analysis())
            .into_iter()
            .map(|t| t.term)
            .collect()
    }

    // ------------------------------------------------------------------ stats

    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.reader.searcher();
        let documents = searcher.search(&AllQuery, &Count)? as u64;
        let with_content = searcher.search(
            &TermQuery::new(
                Term::from_field_u64(self.fields.has_content, 1),
                IndexRecordOption::Basic,
            ),
            &Count,
        )? as u64;
        let settings = self.settings();
        let mut roots = Vec::with_capacity(settings.roots.len());
        for r in &settings.roots {
            let key = path_key(&r.path);
            let n = searcher.search(
                &TermQuery::new(
                    Term::from_field_text(self.fields.root, &key),
                    IndexRecordOption::Basic,
                ),
                &Count,
            )? as u64;
            roots.push(RootStats {
                path: key,
                enabled: r.enabled,
                exists: r.path.exists(),
                documents: n,
            });
        }
        Ok(IndexStats {
            documents,
            with_content,
            index_size_bytes: dir_size(&self.paths.index_dir)
                + dir_size(&self.paths.data_dir.join("manifest.redb")),
            last_indexed: self
                .manifest
                .get_meta("last_indexed")?
                .and_then(|v| v.parse().ok()),
            roots,
            indexing: self.running.load(Ordering::SeqCst),
            watching: self.is_watching(),
            schema_version: SCHEMA_VERSION,
            data_dir: self.paths.data_dir.display().to_string(),
        })
    }

    /// Number of documents currently searchable.
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }
}

fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn truncate_chars(s: &str, max: usize) -> (&str, bool) {
    match s.char_indices().nth(max) {
        Some((i, _)) => (&s[..i], true),
        None => (s, false),
    }
}

fn segments_from_ranges(text: &str, ranges: &[std::ops::Range<usize>]) -> Vec<PreviewSegment> {
    let mut out = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut cursor = 0;
    for r in ranges {
        if r.start > cursor {
            out.push(PreviewSegment {
                text: text[cursor..r.start].to_string(),
                highlight: false,
            });
        }
        out.push(PreviewSegment {
            text: text[r.start..r.end].to_string(),
            highlight: true,
        });
        cursor = r.end;
    }
    if cursor < text.len() {
        out.push(PreviewSegment {
            text: text[cursor..].to_string(),
            highlight: false,
        });
    }
    out
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.stop_watching();
    }
}

#[allow(dead_code)]
fn _assert_mode(_: AnalysisMode) {}
