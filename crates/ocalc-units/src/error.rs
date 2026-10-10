//! Unit errors.

use thiserror::Error;

/// Errors from unit conversion.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum UnitError {
    /// Unknown unit symbol.
    #[error("unknown unit: {0}")]
    UnknownUnit(String),
    /// Units belong to different categories.
    #[error("incompatible units: {from} ({from_cat}) vs {to} ({to_cat})")]
    Incompatible {
        /// Source symbol.
        from: String,
        /// Source category.
        from_cat: String,
        /// Target symbol.
        to: String,
        /// Target category.
        to_cat: String,
    },
    /// Invalid value (e.g. below absolute zero).
    #[error("invalid value for unit {unit}: {message}")]
    InvalidValue {
        /// Unit symbol.
        unit: String,
        /// Reason.
        message: String,
    },
}
