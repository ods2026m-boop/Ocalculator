//! `ocalc-matrix`: matrix and vector foundation for Ocalculator.
//!
//! Dense row-major [`Matrix`] and [`Vector`] of `f64` with basic operations
//! (add/sub/mul, transpose, determinant, inverse, dot/norm/cross). Exact
//! [`Number`](ocalc_number::Number) scalars convert through `to_f64` via
//! [`Matrix::from_numbers`] / [`Vector::from_numbers`].
//!
//! This crate is the substrate for `ocalc-solver` (linear systems, equation
//! and polynomial solvers) and, later, the Iced graphing UI. It performs no
//! parsing and knows nothing about the engine.

mod error;
mod matrix;
mod vector;

pub use error::MatrixError;
pub use matrix::Matrix;
pub use vector::Vector;
