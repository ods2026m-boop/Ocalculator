//! History errors.

use thiserror::Error;

/// Errors from history storage.
#[derive(Debug, Error)]
pub enum HistoryError {
    /// JSON (de)serialisation failed.
    #[error("storage error: {0}")]
    Storage(String),
    /// File I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
