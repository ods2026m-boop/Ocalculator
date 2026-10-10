//! `ocalc-functions`: mathematical functions, trigonometry, logarithms,
//! statistics and constants.
//!
//! All functions operate on [`ocalc_number::Number`] and return
//! `Result<Number, FunctionError>` so the engine can map failures to
//! [`CalcError`](ocalc_engine_like_error) uniformly.
//!
//! Trig functions respect [`AngleMode`] (radians by default, degrees on
//! request) — the engine owns one mode per [`Context`](crate::FunctionContext).

mod constants;
mod error;
mod modules;
mod registry;
mod stats;
mod trig;

pub use constants::{lookup_constant, CONSTANTS};
pub use error::FunctionError;
pub use modules::{ScientificModule, StandardMathModule, StatisticsModule};
pub use registry::{
    default_registry, scientific_registry, standard_registry, statistics_registry, FunctionContext,
    FunctionRegistry, MathFn, MathFunction,
};
pub use stats::{mean, median, stddev, sum, variance};
pub use trig::AngleMode;
