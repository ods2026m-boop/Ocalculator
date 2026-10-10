//! AST optimization: constant folding + safe algebraic identities.
//!
//! [`optimize`] rewrites an [`Expr`] bottom-up:
//! - folds `Number`-only arithmetic (`2 + 3*4` → `14`) using [`Number`] ops,
//! - applies identity rules (`x + 0` → `x`, `x * 1` → `x`, `-(-x)` → `x`),
//! - never changes semantics: fallible folds (`1/0`, `(-1)!`) are left unfolded
//!   so evaluation still reports the error with the original call-site span,
//! - never folds `Call` nodes (function purity/angle-mode belong to the
//!   function layer, not the AST layer).
//!
//! Deliberately *not* folded: `0 * x` → `0` (wrong when `x` is `inf`/`NaN` at
//! runtime) and `0 / x`, `x - x` (same NaN hazard). Only identities valid for
//! all IEEE/extended values are applied.

use ocalc_number::Number;

use crate::{BinOp, Expr, ExprKind, UnOp};

/// Optimize an expression (constant folding + identities).
///
/// Pure — returns a new [`Expr`]; the input is consumed.
#[must_use]
pub fn optimize(expr: Expr) -> Expr {
    fold(expr)
}

/// Alias for [`optimize`] (the only optimization pass in Phase 2).
#[must_use]
pub fn fold_constants(expr: Expr) -> Expr {
    fold(expr)
}

fn fold(expr: Expr) -> Expr {
    match expr.kind {
        ExprKind::Number(_) | ExprKind::Var(_) => expr,
        ExprKind::Assign { name, expr: rhs } => {
            let rhs = fold(*rhs);
            let span = expr.span;
            Expr::new(
                ExprKind::Assign {
                    name,
                    expr: Box::new(rhs),
                },
                span,
            )
        }
        ExprKind::Call { name, args } => {
            let args = args.into_iter().map(fold).collect();
            Expr::new(ExprKind::Call { name, args }, expr.span)
        }
        ExprKind::Unary { op, expr: inner } => {
            let inner = fold(*inner);
            fold_unary(op, inner, expr.span)
        }
        ExprKind::Factorial(inner) => {
            let inner = fold(*inner);
            match &inner.kind {
                ExprKind::Number(n) => match n.factorial() {
                    Ok(v) => Expr::new(ExprKind::Number(v), expr.span),
                    Err(_) => Expr::new(ExprKind::Factorial(Box::new(inner)), expr.span),
                },
                _ => Expr::new(ExprKind::Factorial(Box::new(inner)), expr.span),
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left = fold(*left);
            let right = fold(*right);
            fold_binary(op, left, right, expr.span)
        }
    }
}

fn fold_unary(op: UnOp, inner: Expr, span: crate::Span) -> Expr {
    // `--x` → `x`, `+x` → `x`, `~~x` → `x` (inner already folded).
    match (&op, &inner.kind) {
        (UnOp::Plus, _) => inner,
        (
            UnOp::Neg,
            ExprKind::Unary {
                op: UnOp::Neg,
                expr: x,
            },
        ) => x.as_ref().clone(),
        (
            UnOp::BitNot,
            ExprKind::Unary {
                op: UnOp::BitNot,
                expr: x,
            },
        ) => x.as_ref().clone(),
        _ => match &inner.kind {
            ExprKind::Number(n) => {
                let v = match op {
                    UnOp::Plus => n.clone(),
                    UnOp::Neg => n.neg(),
                    UnOp::BitNot => match n.bitnot() {
                        Ok(v) => v,
                        Err(_) => {
                            return Expr::new(
                                ExprKind::Unary {
                                    op,
                                    expr: Box::new(inner),
                                },
                                span,
                            )
                        }
                    },
                };
                Expr::new(ExprKind::Number(v), span)
            }
            _ => Expr::new(
                ExprKind::Unary {
                    op,
                    expr: Box::new(inner),
                },
                span,
            ),
        },
    }
}

fn is_zero(n: &Number) -> bool {
    n.is_zero()
}

fn is_one(n: &Number) -> bool {
    *n == Number::Integer(1) || *n == Number::Float(1.0)
}

fn fold_binary(op: BinOp, left: Expr, right: Expr, span: crate::Span) -> Expr {
    // Safe identities (valid for all values incl. inf/NaN).
    match op {
        BinOp::Add => {
            if let ExprKind::Number(n) = &right.kind {
                if is_zero(n) {
                    return left;
                }
            }
            if let ExprKind::Number(n) = &left.kind {
                if is_zero(n) {
                    return right;
                }
            }
        }
        BinOp::Sub => {
            if let ExprKind::Number(n) = &right.kind {
                if is_zero(n) {
                    return left;
                }
            }
        }
        BinOp::Mul => {
            if let ExprKind::Number(n) = &right.kind {
                if is_one(n) {
                    return left;
                }
            }
            if let ExprKind::Number(n) = &left.kind {
                if is_one(n) {
                    return right;
                }
            }
        }
        BinOp::Div => {
            if let ExprKind::Number(n) = &right.kind {
                if is_one(n) {
                    return left;
                }
            }
        }
        BinOp::Pow => {
            if let ExprKind::Number(n) = &right.kind {
                if is_one(n) {
                    return left;
                }
                if is_zero(n) {
                    // x^0 = 1 (matches `Number::pow`: 0^0 folds to 1 too).
                    return Expr::new(ExprKind::Number(Number::Integer(1)), span);
                }
            }
        }
        _ => {}
    }

    // Constant folding when both sides are literals.
    if let (ExprKind::Number(l), ExprKind::Number(r)) = (&left.kind, &right.kind) {
        let folded = match op {
            BinOp::Add => l.add(r),
            BinOp::Sub => l.sub(r),
            BinOp::Mul => l.mul(r),
            BinOp::Div => l.div(r),
            BinOp::Mod => l.rem(r),
            BinOp::Pow => l.pow(r),
            BinOp::BitAnd => l.bitand(r),
            BinOp::BitOr => l.bitor(r),
            BinOp::BitXor => l.bitxor(r),
            BinOp::Shl => l.shl(r),
            BinOp::Shr => l.shr(r),
        };
        if let Ok(v) = folded {
            return Expr::new(ExprKind::Number(v), span);
        }
    }
    Expr::new(
        ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        span,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    fn opt(src: &str) -> Expr {
        optimize(Parser::parse_source(src).unwrap())
    }

    fn is_num(e: &Expr) -> bool {
        matches!(e.kind, ExprKind::Number(_))
    }

    #[test]
    fn folds_arithmetic() {
        assert!(is_num(&opt("2 + 3 * 4"))); // 14
        assert_eq!(
            opt("2 + 3 * 4").kind,
            Parser::parse_source("14").unwrap().kind
        );
        assert!(is_num(&opt("(1 + 2) * (3 + 4)")));
        assert!(is_num(&opt("2^10"))); // 1024
        assert!(is_num(&opt("5!"))); // 120
        assert!(is_num(&opt("--5"))); // 5
        assert!(is_num(&opt("0xFF + 1"))); // 256
    }

    #[test]
    fn identities() {
        assert!(matches!(opt("x + 0").kind, ExprKind::Var(_)));
        assert!(matches!(opt("x * 1").kind, ExprKind::Var(_)));
        assert!(matches!(opt("x - 0").kind, ExprKind::Var(_)));
        assert!(matches!(opt("x / 1").kind, ExprKind::Var(_)));
        assert!(matches!(opt("x ^ 1").kind, ExprKind::Var(_)));
        assert!(matches!(opt("x ^ 0").kind, ExprKind::Number(_)));
        assert!(matches!(opt("-(-x)").kind, ExprKind::Var(_)));
    }

    #[test]
    fn does_not_fold_what_it_shouldnt() {
        // Variables and calls survive.
        assert!(matches!(opt("x + 1").kind, ExprKind::Binary { .. }));
        assert!(matches!(opt("sin(0)").kind, ExprKind::Call { .. }));
        // Fallible folds are left for runtime errors (with spans).
        assert!(matches!(opt("1/0").kind, ExprKind::Binary { .. }));
        assert!(matches!(opt("(-1)!").kind, ExprKind::Factorial(_)));
        // NaN-hazardous identities are NOT applied.
        assert!(matches!(opt("0 * x").kind, ExprKind::Binary { .. }));
        assert!(matches!(opt("x - x").kind, ExprKind::Binary { .. }));
    }

    #[test]
    fn fold_is_idempotent_and_span_preserving() {
        let once = opt("2 + 3 * 4 + x");
        let twice = optimize(once.clone());
        assert_eq!(once.kind, twice.kind);
        // Folded literal keeps the merged span of its inputs.
        let e = opt("1 + 2");
        assert!(e.span.end > e.span.start);
    }

    #[test]
    fn optimized_eval_matches_raw_eval() {
        use crate::Calculator;
        for src in ["1+2*3", "2^3^2", "5! + 1", "x*2+1", "sqrt(4)+1"] {
            let mut a = Calculator::new();
            a.context_mut().set_var("x", Number::Integer(7));
            let mut b = Calculator::new();
            b.context_mut().set_var("x", Number::Integer(7));
            let raw = crate::Parser::parse_source(src).unwrap();
            let opt_e = optimize(raw.clone());
            // sqrt(4) is a Call → not folded; evals must still agree.
            assert_eq!(a.eval_expr(&raw).ok(), b.eval_expr(&opt_e).ok(), "{src}");
        }
    }
}
