//! Stable engine configuration and resource limits (Phase 3).
//!
//! [`EngineLimits`] guards every untrusted input: expression length, token
//! count, parser/evaluator recursion depth, function arity and graph size.
//! Defaults are generous for interactive use but block denial-of-service
//! shapes (megabyte pastes, 100k-deep parens, million-point graphs).
//!
//! Frontends (Iced, CLI, ODS-os tools) share these defaults via
//! [`EngineConfig::default`]; sandboxes tighten them via
//! [`EngineConfig::sandbox`].

use serde::{Deserialize, Serialize};

use crate::{AngleMode, CalcError};

/// Resource limits enforced before and during evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineLimits {
    /// Maximum source length in bytes (default 32 768).
    pub max_expression_len: usize,
    /// Maximum lexer tokens including trailing `Eof` (default 8 192).
    pub max_tokens: usize,
    /// Maximum parser nesting depth: parens + calls + unary chains
    /// (default 128).
    pub max_recursion_depth: usize,
    /// Maximum evaluator nesting depth (default 1024).
    pub max_eval_depth: usize,
    /// Maximum arguments to a single call (default 128).
    pub max_function_args: usize,
    /// Maximum graph sample points per call (default 100 000).
    pub max_graph_points: usize,
    /// Maximum live variables in a context (default 1 024).
    pub max_variables: usize,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_expression_len: 32_768,
            max_tokens: 8_192,
            max_recursion_depth: 128,
            max_eval_depth: 1_024,
            max_function_args: 128,
            max_graph_points: 100_000,
            max_variables: 1_024,
        }
    }
}

impl EngineLimits {
    /// Tight limits for untrusted / sandboxed inputs.
    #[must_use]
    pub fn sandbox() -> Self {
        Self {
            max_expression_len: 4_096,
            max_tokens: 1_024,
            max_recursion_depth: 32,
            max_eval_depth: 64,
            max_function_args: 32,
            max_graph_points: 10_000,
            max_variables: 128,
        }
    }

    /// Validate the limit values themselves (all must be ≥ 1, with sane caps).
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] when any limit is zero or absurd.
    pub fn validate(&self) -> Result<(), CalcError> {
        let checks = [
            ("max_expression_len", self.max_expression_len, 1_000_000),
            ("max_tokens", self.max_tokens, 500_000),
            ("max_recursion_depth", self.max_recursion_depth, 10_000),
            ("max_eval_depth", self.max_eval_depth, 10_000),
            ("max_function_args", self.max_function_args, 100_000),
            ("max_graph_points", self.max_graph_points, 10_000_000),
            ("max_variables", self.max_variables, 100_000),
        ];
        for (name, value, cap) in checks {
            if value == 0 {
                return Err(CalcError::limit(format!("{name} must be >= 1")));
            }
            if value > cap {
                return Err(CalcError::limit(format!(
                    "{name}={value} exceeds hard cap {cap}"
                )));
            }
        }
        Ok(())
    }

    /// Reject an over-long source string.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] with a 0-width span at the limit offset.
    pub fn check_source_len(&self, src: &str) -> Result<(), CalcError> {
        if src.len() > self.max_expression_len {
            return Err(CalcError::LimitExceeded {
                message: format!(
                    "expression is {} bytes, limit is {}",
                    src.len(),
                    self.max_expression_len
                ),
                span: Some(crate::Span::point(self.max_expression_len.min(src.len()))),
            });
        }
        Ok(())
    }

    /// Reject an over-large token stream.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`].
    pub fn check_token_count(&self, n: usize) -> Result<(), CalcError> {
        if n > self.max_tokens {
            return Err(CalcError::limit(format!(
                "expression produced {n} tokens, limit is {}",
                self.max_tokens
            )));
        }
        Ok(())
    }

    /// Reject an over-large argument list.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`].
    pub fn check_arg_count(&self, n: usize) -> Result<(), CalcError> {
        if n > self.max_function_args {
            return Err(CalcError::limit(format!(
                "call has {n} arguments, limit is {}",
                self.max_function_args
            )));
        }
        Ok(())
    }

    /// Reject an over-large graph request.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`].
    pub fn check_graph_points(&self, n: usize) -> Result<(), CalcError> {
        if n > self.max_graph_points {
            return Err(CalcError::limit(format!(
                "graph requested {n} points, limit is {}",
                self.max_graph_points
            )));
        }
        Ok(())
    }
}

/// Top-level engine configuration: angle mode + resource limits.
///
/// This is the only knob a frontend needs besides module registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Trig angle mode.
    pub angle_mode: AngleMode,
    /// Resource limits.
    pub limits: EngineLimits,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            angle_mode: AngleMode::Radians,
            limits: EngineLimits::default(),
        }
    }
}

impl EngineConfig {
    /// Default configuration (radians + default limits).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Degrees + default limits.
    #[must_use]
    pub fn with_degrees() -> Self {
        Self {
            angle_mode: AngleMode::Degrees,
            ..Self::default()
        }
    }

    /// Sandboxed configuration (radians + tight limits).
    #[must_use]
    pub fn sandbox() -> Self {
        Self {
            angle_mode: AngleMode::Radians,
            limits: EngineLimits::sandbox(),
        }
    }

    /// Validate limits.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] for degenerate limits.
    pub fn validate(&self) -> Result<(), CalcError> {
        self.limits.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        EngineLimits::default().validate().unwrap();
        EngineLimits::sandbox().validate().unwrap();
        EngineConfig::default().validate().unwrap();
    }

    #[test]
    fn rejects_degenerate() {
        let l = EngineLimits {
            max_tokens: 0,
            ..EngineLimits::default()
        };
        assert!(l.validate().is_err());
    }

    #[test]
    fn source_len_guard() {
        let l = EngineLimits {
            max_expression_len: 4,
            ..EngineLimits::default()
        };
        assert!(l.check_source_len("12345").is_err());
        assert!(l.check_source_len("1234").is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let c = EngineConfig::default();
        let j = serde_json::to_string(&c).unwrap();
        let back: EngineConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(c, back);
    }
}
