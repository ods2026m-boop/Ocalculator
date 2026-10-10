//! Solver errors.
use thiserror::Error;

/// Failures from equation / polynomial / linear-system solvers.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SolverError {
    /// Degenerate input (e.g. `0x = 5`, empty polynomial).
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// System has no (unique) solution.
    #[error("no solution: {0}")]
    NoSolution(String),
    /// Iterative method did not converge in the iteration budget.
    #[error("no convergence after {iterations} iterations: {detail}")]
    NoConvergence {
        /// Iterations spent.
        iterations: usize,
        /// Extra detail.
        detail: String,
    },
    /// Underlying matrix failure (singular, dimension mismatch, …).
    #[error("linear algebra failure: {0}")]
    Linalg(String),
}
