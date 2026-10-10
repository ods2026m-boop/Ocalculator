//! `ocalc-cas`: computer algebra system for Ocalculator.
//!
//! Symbolic expressions ([`SymExpr`]) with a simplification engine
//! (`x + x → 2*x`, `x*x → x^2`), expansion (`(x+1)^2 → x^2+2*x+1`),
//! factorization foundation, substitution, differentiation
//! (`d/dx x^2 = 2*x`), integration foundation (`integral x dx = x^2/2`),
//! and equation solving (`x + 5 = 10`, `x^2 - 4 = 0`).
//!
//! The symbolic API is native Rust (engine expressions evaluate to numbers
//! and cannot carry trees). The [`CasModule`] bridges the numeric kernels
//! (`poly_eval`, `solve_linear`, `quadratic_disc`, `quadratic_root`) into
//! engine expressions via [`FunctionProvider`](ocalc_modules::FunctionProvider).

mod calculus;
mod expand;
mod expr;
mod factor;
mod module;
mod simplify;
mod solve;
mod substitute;
mod symbol;

pub use calculus::{differentiate, integrate};
pub use expand::expand;
pub use expr::{add, cos, konst, log, mul, pow, sin, var, var_of, FuncKind, SymExpr};
pub use factor::factor;
pub use module::{CasModule, CasProvider};
pub use simplify::simplify;
pub use solve::{discriminant, solution_exprs, solve_const_eq, solve_linear, solve_quadratic};
pub use substitute::substitute;
pub use symbol::Symbol;
