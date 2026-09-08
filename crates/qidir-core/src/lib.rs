//! # qidir-core
//!
//! Search-engine core of **QIDIR** — a modern local full-text file search
//! application for Windows with first-class support for Russian, Kazakh and
//! English.
//!
//! The crate is UI-agnostic and is shared by the Tauri desktop application and
//! the `qidir` command line tool. It provides:
//!
//! * [`analysis`] — multilingual text analysis (tokenization, character folding,
//!   language detection per token, Russian/English Snowball stemming and a
//!   light-weight Kazakh stemmer).
//! * [`extract`] — text extraction from dozens of file formats (plain text in
//!   any legacy encoding, DOCX/XLSX/PPTX, ODF, legacy DOC/XLS, PDF, RTF, HTML,
//!   EPUB, FB2, source code, ...).
//! * [`index`] — a [tantivy] based inverted index with incremental updates,
//!   parallel crawling and a filesystem watcher.
//! * [`search`] — a forgiving query language (phrases, prefixes, fuzzy terms,
//!   exclusions, field filters) plus snippet generation and preview highlighting.
//! * [`engine::Engine`] — a thread-safe facade tying everything together.

pub mod analysis;
pub mod config;
pub mod engine;
pub mod error;
pub mod extract;
pub mod fs_util;
pub mod index;
pub mod search;

pub use config::Settings;
pub use engine::{Engine, IndexProgress, IndexState};
pub use error::{Error, Result};
pub use search::{SearchHit, SearchRequest, SearchResponse, SortOrder};

/// Crate version as compiled.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
