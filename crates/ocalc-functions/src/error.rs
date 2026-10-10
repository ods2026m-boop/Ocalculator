//! Function-layer errors.

use thiserror::Error;

/// Errors raised by mathematical functions.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FunctionError {
    /// Wrong number of arguments.
    #[error("function '{name}' expects {expected} argument(s), got {got}")]
    Arity {
        /// Function name.
        name: String,
        /// Expected arity (or minimum).
        expected: usize,
        /// What was supplied.
        got: usize,
    },
    /// Domain error (e.g. `sqrt` handled elsewhere, `log(-1)` real-mode).
    #[error("domain error in '{name}': {message}")]
    Domain {
        /// Function name.
        name: String,
        /// Human-readable reason.
        message: String,
    },
    /// Underlying number failure.
    #[error(transparent)]
    Number(#[from] ocalc_number::NumberError),
    /// Unknown function (also surfaced by the engine with spans).
    #[error("unknown function: {0}")]
    Unknown(String),
}
