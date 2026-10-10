//! Scalar root-finding: bisection + Newton.
//!
//! Numeric complements to the exact [`crate::Polynomial`] solvers, for
//! higher-degree or transcendental equations (e.g. `x - cos(x) = 0`).

use crate::SolverError;

/// Bisection on `[a, b]`. Requires a sign change (`f(a)*f(b) ≤ 0`).
pub fn bisection(
    mut f: impl FnMut(f64) -> f64,
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, SolverError> {
    if !(a.is_finite() && b.is_finite() && tol > 0.0) {
        return Err(SolverError::InvalidInput(
            "bisection needs finite a/b and tol > 0".to_owned(),
        ));
    }
    let (mut fa, fb) = (f(a), f(b));
    if !(fa.is_finite() && fb.is_finite()) {
        return Err(SolverError::InvalidInput(
            "f(a)/f(b) must be finite".to_owned(),
        ));
    }
    if fa == 0.0 {
        return Ok(a);
    }
    if fb == 0.0 {
        return Ok(b);
    }
    if fa.signum() == fb.signum() {
        return Err(SolverError::NoSolution(format!(
            "no sign change: f({a})={fa}, f({b})={fb}"
        )));
    }
    for i in 0..max_iter {
        let m = 0.5 * (a + b);
        let fm = f(m);
        if !fm.is_finite() {
            return Err(SolverError::InvalidInput(format!("f({m}) is not finite")));
        }
        if fm == 0.0 || (b - a) / 2.0 < tol {
            return Ok(m);
        }
        if fa.signum() == fm.signum() {
            a = m;
            fa = fm;
        } else {
            b = m;
        }
        let _ = i;
    }
    Err(SolverError::NoConvergence {
        iterations: max_iter,
        detail: format!("interval [{a}, {b}]"),
    })
}

/// Newton's method from `x0` with explicit derivative `df`.
pub fn newton(
    mut f: impl FnMut(f64) -> f64,
    mut df: impl FnMut(f64) -> f64,
    mut x: f64,
    tol: f64,
    max_iter: usize,
) -> Result<f64, SolverError> {
    if !(x.is_finite() && tol > 0.0) {
        return Err(SolverError::InvalidInput(
            "newton needs finite x0 and tol > 0".to_owned(),
        ));
    }
    for i in 0..max_iter {
        let (fx, dfx) = (f(x), df(x));
        if !(fx.is_finite() && dfx.is_finite()) {
            return Err(SolverError::InvalidInput(format!("f/df not finite at {x}")));
        }
        if dfx == 0.0 {
            return Err(SolverError::NoSolution(format!("zero derivative at {x}")));
        }
        let next = x - fx / dfx;
        if !next.is_finite() {
            return Err(SolverError::InvalidInput("newton step diverged".to_owned()));
        }
        if (next - x).abs() < tol {
            return Ok(next);
        }
        x = next;
        let _ = i;
    }
    Err(SolverError::NoConvergence {
        iterations: max_iter,
        detail: format!("last x = {x}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisects_sqrt2() {
        let r = bisection(|x| x * x - 2.0, 0.0, 2.0, 1e-12, 100).unwrap();
        assert!((r - 2f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn rejects_missing_bracket() {
        assert!(bisection(|x| x * x + 1.0, 0.0, 2.0, 1e-9, 50).is_err());
    }

    #[test]
    fn newton_finds_root() {
        let r = newton(|x| x * x - 2.0, |x| 2.0 * x, 1.0, 1e-12, 50).unwrap();
        assert!((r - 2f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn newton_zero_derivative_errors() {
        assert!(newton(|x| x * x, |_| 0.0, 1.0, 1e-9, 10).is_err());
    }
}
