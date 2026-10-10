//! Expression system: parsed, reusable, inspectable expressions.

use serde::{Deserialize, Serialize};

use ocalc_number::Number;

use crate::{CalcError, Context, EngineLimits, Expr};

/// Serialization format version for saved expressions (Phase 3, v1).
pub const EXPRESSION_FORMAT_VERSION: u32 = 1;

/// A parsed expression that can be evaluated repeatedly (e.g. graph sampling
/// binds a different `x` per point without re-parsing).
///
/// This is part of the **stable public API**: frontends parse once via
/// [`Expression::parse`] / [`Expression::parse_limited`] and evaluate many
/// times via [`Expression::evaluate`]. The type is serializable (versioned
/// JSON) so expressions and history entries survive restarts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Expression {
    source: String,
    ast: Expr,
    #[serde(default = "default_expression_format_version")]
    format_version: u32,
}

fn default_expression_format_version() -> u32 {
    EXPRESSION_FORMAT_VERSION
}

impl Expression {
    /// Parse `source` without optimization (raw AST, original spans).
    ///
    /// # Errors
    /// Lexer / parser [`CalcError`]s.
    pub fn parse(source: &str) -> Result<Self, CalcError> {
        Self::parse_limited(source, &EngineLimits::default())
    }

    /// Parse with explicit resource limits (Phase 3 hardening).
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] on resource violations, else lexer /
    /// parser errors.
    pub fn parse_limited(source: &str, limits: &EngineLimits) -> Result<Self, CalcError> {
        let ast = crate::Parser::parse_source_limited(source, limits)?;
        Ok(Self {
            source: source.to_owned(),
            ast,
            format_version: EXPRESSION_FORMAT_VERSION,
        })
    }

    /// Parse `source` and run [`crate::optimize`] (constant folding).
    ///
    /// Prefer this for one-shot evaluation; use [`Self::parse`] when the raw
    /// AST is needed for highlighting or debugging.
    ///
    /// # Errors
    /// Lexer / parser [`CalcError`]s.
    pub fn parse_optimized(source: &str) -> Result<Self, CalcError> {
        Self::parse_optimized_limited(source, &EngineLimits::default())
    }

    /// [`Self::parse_optimized`] with explicit limits.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] or lexer / parser errors.
    pub fn parse_optimized_limited(source: &str, limits: &EngineLimits) -> Result<Self, CalcError> {
        let ast = crate::Parser::parse_source_limited(source, limits)?;
        Ok(Self {
            source: source.to_owned(),
            ast: crate::optimize(ast),
            format_version: EXPRESSION_FORMAT_VERSION,
        })
    }

    /// Rebuild from an already-parsed AST (e.g. after programmatic AST edits).
    #[must_use]
    pub fn from_ast(source: &str, ast: Expr) -> Self {
        Self {
            source: source.to_owned(),
            ast,
            format_version: EXPRESSION_FORMAT_VERSION,
        }
    }

    /// Optimize the stored AST in place (idempotent).
    pub fn optimize(&mut self) {
        let ast = std::mem::replace(
            &mut self.ast,
            crate::Expr::new(
                crate::ExprKind::Number(ocalc_number::Number::Integer(0)),
                crate::Span::point(0),
            ),
        );
        self.ast = crate::optimize(ast);
    }

    /// Consume into an optimized copy.
    #[must_use]
    pub fn into_optimized(mut self) -> Self {
        self.optimize();
        self
    }

    /// Original source text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Borrow the AST (for inspection, optimisation, UI highlighting).
    #[must_use]
    pub fn ast(&self) -> &Expr {
        &self.ast
    }

    /// Format version of this payload (always [`EXPRESSION_FORMAT_VERSION`]).
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Evaluate in `ctx` (mutates variables on assignment).
    ///
    /// # Errors
    /// Evaluation [`CalcError`]s.
    pub fn evaluate(&self, ctx: &mut Context) -> Result<Number, CalcError> {
        crate::evaluator::eval(&self.ast, ctx)
    }

    /// Evaluate with an explicit evaluation-depth guard.
    ///
    /// # Errors
    /// [`CalcError::LimitExceeded`] when the AST exceeds `max_depth`, else
    /// evaluation errors.
    pub fn evaluate_limited(
        &self,
        ctx: &mut Context,
        max_depth: usize,
    ) -> Result<Number, CalcError> {
        crate::evaluator::eval_limited(&self.ast, ctx, max_depth)
    }

    /// Parse and evaluate in one shot with a fresh default context.
    ///
    /// # Errors
    /// Any [`CalcError`].
    pub fn evaluate_fresh(source: &str) -> Result<Number, CalcError> {
        let mut ctx = Context::new();
        Self::parse(source)?.evaluate(&mut ctx)
    }

    /// Serialize to versioned JSON (`{"format_version":1,…}`).
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "format_version": EXPRESSION_FORMAT_VERSION,
            "source": self.source,
            "ast": self.ast,
        }))
        .unwrap_or_else(|_| {
            format!(
                "{{\"format_version\":{EXPRESSION_FORMAT_VERSION},\"source\":{}}}",
                serde_json::to_string(&self.source).unwrap_or_default()
            )
        })
    }

    /// Deserialize [`Self::to_json`] output.
    ///
    /// Accepts both enveloped payloads and bare `{"source":…,"ast":…}`
    /// objects; when only `source` is present the AST is re-parsed. Unknown
    /// future `format_version` values are rejected.
    ///
    /// # Errors
    /// Human-readable strings (JSON or version errors).
    pub fn from_json(s: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| format!("invalid expression JSON: {e}"))?;
        if let Some(ver) = v.get("format_version").and_then(|n| n.as_u64()) {
            if ver != u64::from(EXPRESSION_FORMAT_VERSION) {
                return Err(format!(
                    "unsupported expression format version {ver}, expected {}",
                    EXPRESSION_FORMAT_VERSION
                ));
            }
        }
        // Full envelope with AST.
        if v.get("ast").is_some() {
            let source = v
                .get("source")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_owned();
            let ast: Expr = serde_json::from_value(v["ast"].clone())
                .map_err(|e| format!("bad expression AST: {e}"))?;
            return Ok(Self {
                source,
                ast,
                format_version: EXPRESSION_FORMAT_VERSION,
            });
        }
        // Source-only payload: re-parse.
        if let Some(source) = v.get("source").and_then(|s| s.as_str()) {
            return Self::parse(source).map_err(|e| e.to_string());
        }
        Err("expression JSON needs 'source' or 'ast'".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuse_with_different_vars() {
        let e = Expression::parse("x * 2").unwrap();
        let mut ctx = Context::new();
        ctx.set_var("x", Number::Integer(3));
        assert_eq!(e.evaluate(&mut ctx).unwrap(), Number::Integer(6));
        ctx.set_var("x", Number::Integer(10));
        assert_eq!(e.evaluate(&mut ctx).unwrap(), Number::Integer(20));
    }

    #[test]
    fn fresh_eval() {
        assert_eq!(
            Expression::evaluate_fresh("2 + 3 * 4").unwrap(),
            Number::Integer(14)
        );
    }

    #[test]
    fn json_roundtrip_with_ast() {
        let e = Expression::parse("2 + 3 * x").unwrap();
        let j = e.to_json();
        let back = Expression::from_json(&j).unwrap();
        assert_eq!(e, back);
        assert_eq!(back.format_version(), EXPRESSION_FORMAT_VERSION);
    }

    #[test]
    fn json_source_only_reparses() {
        let back = Expression::from_json(r#"{"source": "1 + 2"}"#).unwrap();
        assert_eq!(back.source(), "1 + 2");
    }

    #[test]
    fn rejects_future_version() {
        assert!(
            Expression::from_json(r#"{"format_version": 99, "source": "1", "ast": null}"#).is_err()
        );
    }

    #[test]
    fn limits_enforced() {
        let limits = EngineLimits {
            max_expression_len: 3,
            ..EngineLimits::default()
        };
        assert!(Expression::parse_limited("1 + 2", &limits).is_err());
    }
}
