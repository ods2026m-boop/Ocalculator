//! Polynomials with `f64` coefficients + exact low-degree roots.
//!
//! Coefficients are stored highest-degree first: `[1, 0, -4]` means
//! `x² - 4`. Evaluation uses Horner form. Degrees 1-2 solve exactly
//! (complex roots included); higher degrees use numeric methods in
//! [`crate::equation`].

use num_complex::Complex64;

use crate::SolverError;

/// Polynomial with highest-degree-first coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct Polynomial {
    coeffs: Vec<f64>,
}

impl Polynomial {
    /// Build from highest-first coefficients (leading coefficient nonzero,
    /// at least one coefficient, all finite).
    pub fn new(coeffs: Vec<f64>) -> Result<Self, SolverError> {
        if coeffs.is_empty() {
            return Err(SolverError::InvalidInput(
                "empty coefficient list".to_owned(),
            ));
        }
        if coeffs.iter().any(|c| !c.is_finite()) {
            return Err(SolverError::InvalidInput(
                "coefficients must be finite".to_owned(),
            ));
        }
        let mut start = 0;
        while start + 1 < coeffs.len() && coeffs[start] == 0.0 {
            start += 1;
        }
        Ok(Self {
            coeffs: coeffs[start..].to_vec(),
        })
    }

    /// Build from [`Number`](ocalc_number::Number) coefficients (via `to_f64`).
    pub fn from_numbers(nums: &[ocalc_number::Number]) -> Result<Self, SolverError> {
        Self::new(nums.iter().map(|n| n.to_f64()).collect())
    }

    /// Degree (0 for constants).
    #[must_use]
    pub fn degree(&self) -> usize {
        self.coeffs.len() - 1
    }

    /// Coefficients, highest first.
    #[must_use]
    pub fn coeffs(&self) -> &[f64] {
        &self.coeffs
    }

    /// Evaluate at `x` (Horner).
    #[must_use]
    pub fn eval(&self, x: f64) -> f64 {
        self.coeffs.iter().fold(0.0, |acc, &c| acc * x + c)
    }

    /// Evaluate at a complex point.
    #[must_use]
    pub fn eval_complex(&self, z: Complex64) -> Complex64 {
        self.coeffs
            .iter()
            .fold(Complex64::new(0.0, 0.0), |acc, &c| {
                acc * z + Complex64::new(c, 0.0)
            })
    }

    /// Derivative polynomial.
    #[must_use]
    pub fn derivative(&self) -> Self {
        if self.degree() == 0 {
            return Self { coeffs: vec![0.0] };
        }
        let n = self.degree();
        let d = self
            .coeffs
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, &c)| c * (n - i) as f64)
            .collect();
        Self { coeffs: d }
    }

    /// Solve `a*x + b = 0`. Returns the single root.
    pub fn solve_linear(a: f64, b: f64) -> Result<f64, SolverError> {
        if !a.is_finite() || !b.is_finite() {
            return Err(SolverError::InvalidInput(
                "coefficients must be finite".to_owned(),
            ));
        }
        if a == 0.0 {
            return Err(SolverError::NoSolution(if b == 0.0 {
                "0x = 0 has infinitely many solutions".to_owned()
            } else {
                "0x = b has no solution".to_owned()
            }));
        }
        Ok(-b / a)
    }

    /// Solve `a*x² + b*x + c = 0` → two (possibly complex/repeated) roots.
    pub fn solve_quadratic(a: f64, b: f64, c: f64) -> Result<[Complex64; 2], SolverError> {
        for v in [a, b, c] {
            if !v.is_finite() {
                return Err(SolverError::InvalidInput(
                    "coefficients must be finite".to_owned(),
                ));
            }
        }
        if a == 0.0 {
            let r = Self::solve_linear(b, c)?;
            let z = Complex64::new(r, 0.0);
            return Ok([z, z]);
        }
        let disc = b * b - 4.0 * a * c;
        let two_a = 2.0 * a;
        if disc >= 0.0 {
            let s = disc.sqrt();
            Ok([
                Complex64::new((-b + s) / two_a, 0.0),
                Complex64::new((-b - s) / two_a, 0.0),
            ])
        } else {
            let s = (-disc).sqrt();
            Ok([
                Complex64::new(-b / two_a, s / two_a),
                Complex64::new(-b / two_a, -s / two_a),
            ])
        }
    }

    /// Roots of this polynomial when degree ≤ 2.
    pub fn roots(&self) -> Result<Vec<Complex64>, SolverError> {
        match self.degree() {
            0 => Err(SolverError::NoSolution("non-zero constant has no roots".to_owned())),
            1 => Ok(vec![Complex64::new(Self::solve_linear(self.coeffs[0], self.coeffs[1])?, 0.0)]),
            2 => {
                let [r1, r2] =
                    Self::solve_quadratic(self.coeffs[0], self.coeffs[1], self.coeffs[2])?;
                Ok(vec![r1, r2])
            }
            d => Err(SolverError::NoSolution(format!(
                "exact roots for degree {d} are not implemented; use numeric methods in crate::equation"
            ))),
        }
    }

    /// Real roots only (filters complex pairs with `|im| < tol`).
    pub fn real_roots(&self, tol: f64) -> Result<Vec<f64>, SolverError> {
        Ok(self
            .roots()?
            .into_iter()
            .filter(|z| z.im.abs() <= tol)
            .map(|z| z.re)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_example_x_squared_minus_4() {
        // x² - 4 = 0 → ±2.
        let p = Polynomial::new(vec![1.0, 0.0, -4.0]).unwrap();
        let mut roots = p.real_roots(1e-12).unwrap();
        roots.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(roots.len(), 2);
        assert!((roots[0] + 2.0).abs() < 1e-12);
        assert!((roots[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn quadratic_complex_pair() {
        // x² + 1 = 0 → ±i.
        let [r1, r2] = Polynomial::solve_quadratic(1.0, 0.0, 1.0).unwrap();
        assert!((r1.im - 1.0).abs() < 1e-12);
        assert!((r2.im + 1.0).abs() < 1e-12);
        assert!(r1.re.abs() < 1e-12);
    }

    #[test]
    fn linear_and_degenerate() {
        assert_eq!(Polynomial::solve_linear(2.0, -4.0).unwrap(), 2.0);
        assert!(Polynomial::solve_linear(0.0, 1.0).is_err());
        // a == 0 degrades quadratic → linear (double root report).
        let [r1, r2] = Polynomial::solve_quadratic(0.0, 2.0, -4.0).unwrap();
        assert_eq!(r1.re, 2.0);
        assert_eq!(r2.re, 2.0);
    }

    #[test]
    fn eval_and_derivative() {
        let p = Polynomial::new(vec![2.0, -3.0, 1.0]).unwrap(); // 2x²-3x+1
        assert_eq!(p.eval(1.0), 0.0);
        assert_eq!(p.derivative().coeffs(), &[4.0, -3.0]);
        assert_eq!(
            Polynomial::new(vec![5.0]).unwrap().derivative().coeffs(),
            &[0.0]
        );
    }

    #[test]
    fn validation() {
        assert!(Polynomial::new(vec![]).is_err());
        assert!(Polynomial::new(vec![f64::NAN, 1.0]).is_err());
        // Leading zeros are stripped: [0,0,1,2] → degree 1.
        assert_eq!(
            Polynomial::new(vec![0.0, 0.0, 1.0, 2.0]).unwrap().degree(),
            1
        );
        assert!(Polynomial::new(vec![1.0, 2.0, 3.0, 4.0])
            .unwrap()
            .roots()
            .is_err());
    }
}
