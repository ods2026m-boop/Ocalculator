//! Graph engine: sample `y = f(x)` into render-ready points.
//!
//! The engine never draws — it only computes `Vec<GraphPoint>`; the Iced UI
//! (or any other frontend) renders them later.

use serde::{Deserialize, Serialize};

use ocalc_number::Number;

use crate::{CalcError, Context, EngineLimits, Expression};

/// One sampled point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GraphPoint {
    /// Input coordinate.
    pub x: f64,
    /// Output value (`y`). Non-finite outputs are still returned so the
    /// renderer can break the polyline at discontinuities.
    pub y: f64,
    /// False when `y` is NaN/±inf or evaluation failed.
    pub valid: bool,
}

/// Sample `expr_source` with `var` swept over `[start, end]` in `n` points.
///
/// `n` must be ≥ 2. Each point gets an independent [`Context`] cloned from
/// `base` with `var` bound to the current `x`.
///
/// # Errors
/// [`CalcError::MathError`] when the expression fails to parse, or
/// [`CalcError::MathError`] for invalid ranges. Per-point evaluation
/// failures mark the point invalid instead of aborting.
pub fn sample_expression(
    expr_source: &str,
    var: &str,
    start: f64,
    end: f64,
    n: usize,
    base: &Context,
) -> Result<Vec<GraphPoint>, CalcError> {
    sample_expression_limited(
        expr_source,
        var,
        start,
        end,
        n,
        base,
        &EngineLimits::default(),
    )
}

/// [`sample_expression`] with explicit resource limits (Phase 3 hardening).
///
/// Enforces `max_graph_points`, `max_expression_len` / `max_tokens` (via the
/// single pre-parse) and the evaluator depth guard per point.
///
/// # Errors
/// [`CalcError::LimitExceeded`] on resource violations, else parse/range
/// errors. Per-point failures still mark points invalid instead of aborting.
#[allow(clippy::too_many_arguments)]
pub fn sample_expression_limited(
    expr_source: &str,
    var: &str,
    start: f64,
    end: f64,
    n: usize,
    base: &Context,
    limits: &EngineLimits,
) -> Result<Vec<GraphPoint>, CalcError> {
    if n < 2 {
        return Err(CalcError::MathError {
            message: format!("graph sampling requires at least 2 points, got {n}"),
            span: None,
        });
    }
    limits.check_graph_points(n)?;
    if !start.is_finite() || !end.is_finite() {
        return Err(CalcError::MathError {
            message: "graph range must be finite".to_owned(),
            span: None,
        });
    }
    if var.is_empty() {
        return Err(CalcError::MathError {
            message: "graph variable name must not be empty".to_owned(),
            span: None,
        });
    }
    // Parse once + fold constants (e.g. `2*3 + sin(x)` folds `2*3` a single
    // time instead of per sampled point). The sweep variable is never folded.
    let expr = Expression::parse_optimized_limited(expr_source, limits)?;
    let mut out = Vec::with_capacity(n);
    let step = if n == 1 {
        0.0
    } else {
        (end - start) / (n - 1) as f64
    };
    // Reuse one scratch context instead of cloning the whole base per point:
    // `functions`/`modules` are cheap `Arc` clones done once, and only the
    // variable map is refreshed per iteration. Assignments inside the sampled
    // expression must not leak across points, so any var written during a
    // point (other than the sweep var) is removed afterwards.
    let mut ctx = Context {
        variables: base.variables.clone(),
        functions: base.functions.clone(),
        modules: base.modules.clone(),
        angle_mode: base.angle_mode,
    };
    let base_vars = base.variables.clone();
    for i in 0..n {
        let x = start + step * i as f64;
        // Restore the variable scope to base + sweep var (cheap: only touches
        // entries that changed; typical scopes are < 16 vars).
        ctx.variables.clone_from(&base_vars);
        ctx.set_var(var, Number::Float(x));
        match expr.evaluate_limited(&mut ctx, limits.max_eval_depth) {
            Ok(Number::Complex(c)) => out.push(GraphPoint {
                x,
                y: c.re,
                valid: c.re.is_finite(),
            }),
            Ok(v) => {
                let y = v.to_f64();
                out.push(GraphPoint {
                    x,
                    y,
                    valid: y.is_finite(),
                });
            }
            Err(_) => out.push(GraphPoint {
                x,
                y: f64::NAN,
                valid: false,
            }),
        }
    }
    Ok(out)
}

/// Serialize sampled points to JSON (for ODS-os tools / headless export).
#[must_use]
pub fn points_to_json(points: &[GraphPoint]) -> String {
    serde_json::to_string_pretty(points).unwrap_or_else(|_| "[]".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_line() {
        let base = Context::new();
        let pts = sample_expression("x * 2", "x", 0.0, 4.0, 5, &base).unwrap();
        assert_eq!(pts.len(), 5);
        assert_eq!(
            pts[0],
            GraphPoint {
                x: 0.0,
                y: 0.0,
                valid: true
            }
        );
        assert_eq!(
            pts[4],
            GraphPoint {
                x: 4.0,
                y: 8.0,
                valid: true
            }
        );
    }

    #[test]
    fn samples_sin() {
        let base = Context::new();
        let pts = sample_expression("sin(x)", "x", 0.0, std::f64::consts::PI, 3, &base).unwrap();
        assert!((pts[0].y).abs() < 1e-12);
        assert!((pts[1].y - 1.0).abs() < 1e-9);
        assert!(pts[1].valid);
    }

    #[test]
    fn invalid_range_rejected() {
        let base = Context::new();
        assert!(sample_expression("x", "x", 0.0, 1.0, 1, &base).is_err());
        assert!(sample_expression("x", "x", f64::NAN, 1.0, 5, &base).is_err());
        assert!(sample_expression("???", "x", 0.0, 1.0, 5, &base).is_err());
    }

    #[test]
    fn division_singularity_marked_invalid() {
        let base = Context::new();
        let pts = sample_expression("1/x", "x", -1.0, 1.0, 3, &base).unwrap();
        assert_eq!(pts.len(), 3);
        assert!(!pts[1].valid); // x = 0
    }

    #[test]
    fn graph_point_limit_enforced() {
        let base = Context::new();
        let limits = EngineLimits {
            max_graph_points: 4,
            ..EngineLimits::default()
        };
        assert!(sample_expression_limited("x", "x", 0.0, 1.0, 5, &base, &limits).is_err());
        assert!(sample_expression_limited("x", "x", 0.0, 1.0, 4, &base, &limits).is_ok());
    }

    #[test]
    fn assignments_do_not_leak_across_points() {
        // `y = x * 2` assigns per point; base must not accumulate `y`.
        let base = Context::new();
        let pts = sample_expression("y = x * 2", "x", 0.0, 2.0, 3, &base).unwrap();
        assert_eq!(pts.len(), 3);
        assert!((pts[2].y - 4.0).abs() < 1e-12);
    }

    #[test]
    fn points_json_roundtrips() {
        let pts = vec![GraphPoint {
            x: 0.0,
            y: 1.0,
            valid: true,
        }];
        let j = points_to_json(&pts);
        let back: Vec<GraphPoint> = serde_json::from_str(&j).unwrap();
        assert_eq!(back, pts);
    }
}
