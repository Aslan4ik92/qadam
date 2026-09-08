//! Persistent user settings.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// A folder (or drive) that is indexed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexRoot {
    /// Absolute path, e.g. `D:\Documents`.
    pub path: PathBuf,
    /// Whether this root participates in indexing. Disabled roots keep their
    /// documents in the index but are not re-crawled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Which UI language the desktop app uses. Stored here so both the Tauri app
/// and the CLI can share the file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    #[default]
    Ru,
    Kk,
    En,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// All user-tunable settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Settings schema version for future migrations.
    pub version: u32,
    /// Indexed locations.
    pub roots: Vec<IndexRoot>,
    /// Glob patterns (matched against the full path, `/` or `\` agnostic) that
    /// are skipped entirely.
    pub exclude_globs: Vec<String>,
    /// File extensions (lower-case, without dot) to index. Empty = all
    /// supported formats.
    pub include_extensions: Vec<String>,
    /// File extensions never indexed.
    pub exclude_extensions: Vec<String>,
    /// Files larger than this (bytes) are indexed by name only.
    pub max_file_size_bytes: u64,
    /// Store extracted text inside the index to power instant previews and
    /// snippets. Disabling saves disk space but previews re-read files.
    pub store_content: bool,
    /// Index hidden files and folders.
    pub index_hidden: bool,
    /// Follow symbolic links / junctions while crawling.
    pub follow_links: bool,
    /// Watch roots for changes and update the index in near real time.
    pub watch_changes: bool,
    /// Number of extraction worker threads. `0` = auto.
    pub worker_threads: usize,
    /// Index writer memory budget in megabytes.
    pub writer_memory_mb: usize,
    /// Number of search results per page.
    pub page_size: usize,
    /// UI language.
    pub ui_language: UiLanguage,
    /// UI theme.
    pub theme: Theme,
    /// Re-index automatically on application start.
    pub reindex_on_start: bool,
    /// Extract text from PDF files (can be slow on scanned documents).
    pub extract_pdf: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            roots: Vec::new(),
            exclude_globs: default_exclude_globs(),
            include_extensions: Vec::new(),
            exclude_extensions: Vec::new(),
            max_file_size_bytes: 64 * 1024 * 1024,
            store_content: true,
            index_hidden: false,
            follow_links: false,
            watch_changes: true,
            worker_threads: 0,
            writer_memory_mb: 256,
            page_size: 50,
            ui_language: UiLanguage::Ru,
            theme: Theme::System,
            reindex_on_start: true,
            extract_pdf: true,
        }
    }
}

/// Folders that are practically never useful to search and slow indexing down.
pub fn default_exclude_globs() -> Vec<String> {
    [
        "**/$Recycle.Bin/**",
        "**/System Volume Information/**",
        "**/Windows/**",
        "**/Windows.old/**",
        "**/Program Files/**",
        "**/Program Files (x86)/**",
        "**/ProgramData/**",
        "**/AppData/Local/Temp/**",
        "**/AppData/Local/Microsoft/**",
        "**/AppData/Local/Google/**",
        "**/AppData/Local/Packages/**",
        "**/node_modules/**",
        "**/.git/**",
        "**/.svn/**",
        "**/.hg/**",
        "**/target/**",
        "**/__pycache__/**",
        "**/.venv/**",
        "**/venv/**",
        "**/*.tmp",
        "**/~$*",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Settings {
    /// Load settings from `path`, falling back to defaults when the file does
    /// not exist.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut s: Settings = serde_json::from_str(&text)?;
                s.normalize();
                Ok(s)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    /// Persist settings atomically (write to a temp file and rename).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, json).map_err(|e| Error::io(&tmp, e))?;
        std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))?;
        Ok(())
    }

    /// Clamp values into sane ranges and lower-case extension lists.
    pub fn normalize(&mut self) {
        self.include_extensions = self
            .include_extensions
            .iter()
            .map(|e| e.trim().trim_start_matches('.').to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();
        self.exclude_extensions = self
            .exclude_extensions
            .iter()
            .map(|e| e.trim().trim_start_matches('.').to_lowercase())
            .filter(|e| !e.is_empty())
            .collect();
        self.exclude_globs.retain(|g| !g.trim().is_empty());
        if self.page_size == 0 {
            self.page_size = 50;
        }
        self.page_size = self.page_size.min(1000);
        self.writer_memory_mb = self.writer_memory_mb.clamp(64, 4096);
        if self.max_file_size_bytes == 0 {
            self.max_file_size_bytes = 64 * 1024 * 1024;
        }
        // Deduplicate roots.
        let mut seen = std::collections::HashSet::new();
        self.roots.retain(|r| seen.insert(r.path.clone()));
    }

    /// Effective number of worker threads.
    pub fn effective_workers(&self) -> usize {
        if self.worker_threads == 0 {
            (num_cpus::get().saturating_sub(1)).clamp(1, 16)
        } else {
            self.worker_threads.clamp(1, 64)
        }
    }

    /// Whether `ext` (lower-case, no dot) passes the include/exclude lists.
    pub fn extension_allowed(&self, ext: &str) -> bool {
        if self.exclude_extensions.iter().any(|e| e == ext) {
            return false;
        }
        if self.include_extensions.is_empty() {
            return true;
        }
        self.include_extensions.iter().any(|e| e == ext)
    }
}

/// Where QIDIR keeps its data by default.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub index_dir: PathBuf,
    pub settings_file: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Standard per-user location (`%LOCALAPPDATA%\QIDIR` on Windows).
    pub fn default_paths() -> Result<Self> {
        let base = directories::ProjectDirs::from("kz", "qidir", "QIDIR")
            .map(|d| d.data_local_dir().to_path_buf())
            .ok_or_else(|| Error::Config("cannot determine user data directory".into()))?;
        Ok(Self::in_dir(base))
    }

    /// All data under a custom directory (used by the CLI and tests).
    pub fn in_dir(base: impl Into<PathBuf>) -> Self {
        let base = base.into();
        Self {
            index_dir: base.join("index"),
            settings_file: base.join("settings.json"),
            log_dir: base.join("logs"),
            data_dir: base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.roots.push(IndexRoot { path: PathBuf::from("C:\\Docs"), enabled: true });
        s.include_extensions = vec![".DOCX".into(), "txt".into()];
        s.save(&file).unwrap();
        let loaded = Settings::load(&file).unwrap();
        assert_eq!(loaded.roots, s.roots);
        assert_eq!(loaded.include_extensions, vec!["docx", "txt"]);
        assert!(loaded.extension_allowed("docx"));
        assert!(!loaded.extension_allowed("pdf"));
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::load(&dir.path().join("nope.json")).unwrap();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn tolerates_unknown_and_missing_fields() {
        let s: Settings = serde_json::from_str(r#"{"pageSize": 10, "futureField": 1}"#).unwrap();
        assert_eq!(s.page_size, 10);
        assert!(s.store_content);
    }
}
