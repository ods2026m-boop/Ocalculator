//! Matrix errors.
use thiserror::Error;

/// Failures from [`crate::Matrix`] / [`crate::Vector`] operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum MatrixError {
    /// Shape mismatch for element-wise or multiplicative ops.
    #[error("dimension mismatch: {0}")]
    DimensionMismatch(String),
    /// Operation requires a square matrix.
    #[error("not square: {rows}x{cols}")]
    NotSquare {
        /// Row count.
        rows: usize,
        /// Column count.
        cols: usize,
    },
    /// Matrix is singular (non-invertible / zero pivot).
    #[error("singular matrix")]
    Singular,
    /// Index out of bounds.
    #[error("index out of bounds: ({row}, {col}) in {rows}x{cols}")]
    IndexOutOfBounds {
        /// Requested row.
        row: usize,
        /// Requested column.
        col: usize,
        /// Actual rows.
        rows: usize,
        /// Actual columns.
        cols: usize,
    },
    /// Malformed construction (ragged rows, empty, bad data length).
    #[error("invalid shape: {0}")]
    InvalidShape(String),
}
