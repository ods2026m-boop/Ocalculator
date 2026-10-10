//! Angle handling + arity helpers.

use serde::{Deserialize, Serialize};

use crate::FunctionError;

/// Whether trig functions interpret their argument as radians or degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AngleMode {
    /// Radians (default, mathematically canonical).
    #[default]
    Radians,
    /// Degrees (calculator-style).
    Degrees,
}

impl AngleMode {
    /// Convert a user-supplied angle to radians for the inner `f64` call.
    #[must_use]
    pub fn to_radians(&self, v: f64) -> f64 {
        match self {
            Self::Radians => v,
            Self::Degrees => v.to_radians(),
        }
    }

    /// Convert a radian result back to the user-facing unit (for inverses).
    #[must_use]
    pub fn from_radians(&self, v: f64) -> f64 {
        match self {
            Self::Radians => v,
            Self::Degrees => v.to_degrees(),
        }
    }
}

/// Validate exact argument count.
pub(crate) fn expect_arity(
    name: &str,
    args: &[ocalc_number::Number],
    expected: usize,
) -> Result<(), FunctionError> {
    if args.len() != expected {
        return Err(FunctionError::Arity {
            name: name.to_owned(),
            expected,
            got: args.len(),
        });
    }
    Ok(())
}

/// Validate at-least arity (for variadics like `max`, `sum`).
pub(crate) fn expect_at_least(
    name: &str,
    args: &[ocalc_number::Number],
    min: usize,
) -> Result<(), FunctionError> {
    if args.len() < min {
        return Err(FunctionError::Arity {
            name: name.to_owned(),
            expected: min,
            got: args.len(),
        });
    }
    Ok(())
}
