//! Error types shared by the whole crate.

use std::path::PathBuf;

/// Result alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type of `qidir-core`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("I/O error: {0}")]
    PlainIo(#[from] std::io::Error),

    #[error("index error: {0}")]
    Index(#[from] tantivy::TantivyError),

    #[error("index is opened by another process or is locked")]
    IndexLocked,

    #[error("manifest database error: {0}")]
    Manifest(String),

    #[error("invalid query: {0}")]
    Query(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("unsupported file format: {0}")]
    Unsupported(String),

    #[error("extraction failed: {0}")]
    Extract(String),

    #[error("indexing is already running")]
    AlreadyIndexing,

    #[error("operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io { path: path.into(), source }
    }
}

impl From<redb::Error> for Error {
    fn from(e: redb::Error) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<redb::DatabaseError> for Error {
    fn from(e: redb::DatabaseError) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<redb::TransactionError> for Error {
    fn from(e: redb::TransactionError) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<redb::TableError> for Error {
    fn from(e: redb::TableError) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<redb::StorageError> for Error {
    fn from(e: redb::StorageError) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<redb::CommitError> for Error {
    fn from(e: redb::CommitError) -> Self {
        Error::Manifest(e.to_string())
    }
}
impl From<tantivy::directory::error::OpenDirectoryError> for Error {
    fn from(e: tantivy::directory::error::OpenDirectoryError) -> Self {
        Error::Index(e.into())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Config(e.to_string())
    }
}
impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Other(e.to_string())
    }
}
