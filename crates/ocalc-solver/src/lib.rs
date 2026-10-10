//! `ocalc-solver`: equation and polynomial solver framework.
//!
//! - [`Polynomial`]: exact roots for degree ≤ 2 (`x² - 4 = 0` → `±2`,
//!   `x² + 1 = 0` → `±i`), Horner evaluation, derivatives.
//! - [`equation`]: numeric bisection + Newton for higher-degree and
//!   transcendental equations.
//! - [`linear`]: square linear systems `A·x = b` over [`Matrix`]/[`Vector`].
//!
//! Pure computation over `f64` (+ exact low-degree paths); parsing and
//! engine integration arrive via future `FunctionProvider`s. No UI here.

mod equation;
mod error;
mod linear;
mod polynomial;

pub use equation::{bisection, newton};
pub use error::SolverError;
pub use linear::solve_linear_system;
pub use polynomial::Polynomial;
