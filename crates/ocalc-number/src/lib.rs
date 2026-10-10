//! `ocalc-number`: unified number system for Ocalculator.
//!
//! Provides a single [`Number`] enum covering:
//! - `Integer` (i64, fast path with overflow promotion)
//! - `BigInt` (arbitrary precision via `num-bigint`)
//! - `Float` (f64 IEEE-754)
//! - `Decimal` (28-29 digit decimal via `rust_decimal`, financial/exact decimal)
//! - `Rational` (exact `BigRational`, arbitrary precision numerator/denominator)
//! - `Complex` (complex f64 via `num-complex`)
//!
//! Promotion rules (lowest → highest):
//! `Integer < BigInt < Rational < Decimal < Float < Complex`.
//! Mixed arithmetic promotes to the higher-ranked type.
//!
//! # Precision model
//!
//! - `1 / 3` stays an exact [`BigRational`] (`1/3`), so `1/3 + 1/3 + 1/3 == 1`
//!   exactly. Use [`Number::to_f64`] only when a float is really needed.
//! - `Decimal` is exact base-10 with 28-29 significant digits (ideal for
//!   `0.1 + 0.2 == 0.3`); it is *not* arbitrary precision — [`BigInt`] and
//!   [`BigRational`] are the arbitrary-precision types.
//! - [`Complex`] stores `f64` components; real negatives under `sqrt`/`ln`
//!   produce complex results instead of errors.

mod error;
mod number;
mod ops;

pub use error::NumberError;
pub use num_bigint::BigInt;
pub use num_complex::Complex64;
pub use num_rational::BigRational;
pub use number::Number;
pub use rust_decimal::Decimal;
