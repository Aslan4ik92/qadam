//! Inverted index: schema, incremental indexer, change tracking and watcher.

pub mod indexer;
pub mod manifest;
pub mod schema;
pub mod watcher;

pub use indexer::{IndexJob, IndexSummary, ProgressSink};
pub use manifest::{Fingerprint, Manifest};
pub use schema::{open_or_create_index, Fields};
