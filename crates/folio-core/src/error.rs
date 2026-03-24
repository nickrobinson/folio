//! Error types for the Folio library.

use thiserror::Error;

/// The primary error type for all Folio operations.
#[derive(Debug, Error)]
pub enum FolioError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF parse error at byte offset {offset}: {message}")]
    Parse { offset: u64, message: String },

    #[error("Invalid PDF object: {0}")]
    InvalidObject(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Signature error: {0}")]
    Signature(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Oracle error: {0}")]
    Oracle(String),
}

/// Convenience Result type for Folio operations.
pub type Result<T> = std::result::Result<T, FolioError>;
