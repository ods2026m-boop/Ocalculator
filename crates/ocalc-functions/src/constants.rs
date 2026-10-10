//! Built-in constants.

use ocalc_number::Number;
use std::f64::consts as fc;

/// A named mathematical constant.
#[derive(Debug, Clone, Copy)]
pub struct Constant {
    /// Canonical name (`"pi"`).
    pub name: &'static str,
    /// Value.
    pub value: f64,
    /// Short description.
    pub description: &'static str,
}

/// All built-in constants.
pub const CONSTANTS: &[Constant] = &[
    Constant {
        name: "pi",
        value: fc::PI,
        description: "π ≈ 3.14159",
    },
    Constant {
        name: "e",
        value: fc::E,
        description: "Euler's number ≈ 2.71828",
    },
    Constant {
        name: "tau",
        value: fc::TAU,
        description: "τ = 2π",
    },
    Constant {
        name: "phi",
        value: fc::GOLDEN_RATIO,
        description: "golden ratio",
    },
    Constant {
        name: "sqrt2",
        value: fc::SQRT_2,
        description: "√2",
    },
    Constant {
        name: "sqrt1_2",
        value: fc::FRAC_1_SQRT_2,
        description: "1/√2",
    },
    Constant {
        name: "ln2",
        value: fc::LN_2,
        description: "ln(2)",
    },
    Constant {
        name: "ln10",
        value: fc::LN_10,
        description: "ln(10)",
    },
    Constant {
        name: "log2e",
        value: fc::LOG2_E,
        description: "log₂(e)",
    },
    Constant {
        name: "log10e",
        value: fc::LOG10_E,
        description: "log₁₀(e)",
    },
    Constant {
        name: "inf",
        value: f64::INFINITY,
        description: "positive infinity",
    },
    Constant {
        name: "nan",
        value: f64::NAN,
        description: "not a number",
    },
];

/// Look up a constant by (case-insensitive) name.
#[must_use]
pub fn lookup_constant(name: &str) -> Option<Number> {
    let lower = name.to_ascii_lowercase();
    // Single-letter aliases: π, τ … keep ASCII-only core, map greek too.
    let canonical = match lower.as_str() {
        "π" => "pi",
        "τ" => "tau",
        "φ" => "phi",
        _ => lower.as_str(),
    };
    CONSTANTS
        .iter()
        .find(|c| c.name == canonical)
        .map(|c| Number::Float(c.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_pi_case_insensitive() {
        let v = lookup_constant("PI").unwrap();
        assert!((v.to_f64() - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn unknown_is_none() {
        assert!(lookup_constant("nope").is_none());
    }
}
