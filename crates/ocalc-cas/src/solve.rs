//! Equation solving: exact linear solutions, real quadratic solutions.
//!
//! These are the numeric kernels behind both the native symbolic wrappers
//! and the engine-callable provider functions (`solve_linear`,
//! `quadratic_root`). `x + 5 = 10` and `x^2 - 4 = 0` both reduce to
//! coefficient form first.

use crate::SymExpr;

/// Solve `a·x + b = 0`.
///
/// # Errors
/// `a == 0` (no / infinite solutions).
pub fn solve_linear(a: f64, b: f64) -> Result<f64, String> {
    if a == 0.0 {
        return Err(if b == 0.0 {
            "0 = 0 has infinitely many solutions".to_owned()
        } else {
            format!("{b} = 0 has no solution")
        });
    }
    Ok(-b / a)
}

/// Solve `x + 5 = 10` style equations given as `(lhs, rhs)` constant sides.
///
/// # Errors
/// Non-constant sides or `0 = 0` contradictions (see [`solve_linear`]).
pub fn solve_const_eq(lhs: f64, rhs: f64) -> Result<f64, String> {
    solve_linear(1.0, lhs - rhs).map_err(|_| format!("{lhs} = {rhs} has no solution"))
}

/// Discriminant `b² − 4ac` of `a·x² + b·x + c`.
#[must_use]
pub fn discriminant(a: f64, b: f64, c: f64) -> f64 {
    b * b - 4.0 * a * c
}

/// Real solutions of `a·x² + b·x + c = 0`, ascending.
///
/// Returns an empty vector when the discriminant is negative (complex roots
/// are the engine's [`Number`](ocalc_number::Number) domain, not the CAS
/// real solver's).
///
/// # Errors
/// `a == 0` (not quadratic — use [`solve_linear`]).
pub fn solve_quadratic(a: f64, b: f64, c: f64) -> Result<Vec<f64>, String> {
    if a == 0.0 {
        return Err("not quadratic (a = 0); use solve_linear".to_owned());
    }
    let d = discriminant(a, b, c);
    if d < 0.0 {
        return Ok(Vec::new());
    }
    if d == 0.0 {
        return Ok(vec![-b / (2.0 * a)]);
    }
    let root = d.sqrt();
    let (mut r1, mut r2) = ((-b - root) / (2.0 * a), (-b + root) / (2.0 * a));
    if r1 > r2 {
        std::mem::swap(&mut r1, &mut r2);
    }
    Ok(vec![r1, r2])
}

/// Symbolic solution expressions (`x = value` right-hand sides).
#[must_use]
pub fn solution_exprs(values: &[f64]) -> Vec<SymExpr> {
    values.iter().map(|v| SymExpr::Const(*v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_equations() {
        // x + 5 = 10 → x = 5.
        assert_eq!(solve_const_eq(5.0, 10.0).unwrap(), 5.0);
        assert_eq!(solve_linear(2.0, -4.0).unwrap(), 2.0);
        assert!(solve_linear(0.0, 1.0).is_err());
        assert!(solve_linear(0.0, 0.0).is_err());
    }

    #[test]
    fn quadratics() {
        // x^2 - 4 = 0 → ±2.
        assert_eq!(solve_quadratic(1.0, 0.0, -4.0).unwrap(), vec![-2.0, 2.0]);
        // (x-1)^2 = 0 → 1 (double root).
        assert_eq!(solve_quadratic(1.0, -2.0, 1.0).unwrap(), vec![1.0]);
        // x^2 + 1 = 0 → no real roots.
        assert!(solve_quadratic(1.0, 0.0, 1.0).unwrap().is_empty());
        assert!(solve_quadratic(0.0, 1.0, 1.0).is_err());
        assert_eq!(discriminant(1.0, 0.0, -4.0), 16.0);
        assert_eq!(solution_exprs(&[2.0]).len(), 1);
    }
}
