//! Linear systems `A·x = b` via Gaussian elimination with partial pivoting.

use ocalc_matrix::{Matrix, Vector};

use crate::SolverError;

/// Solve the square system `a·x = b`.
pub fn solve_linear_system(a: &Matrix, b: &Vector) -> Result<Vector, SolverError> {
    let (rows, cols) = a.shape();
    if rows != cols {
        return Err(SolverError::Linalg(format!(
            "A must be square, got {rows}x{cols}"
        )));
    }
    let n = rows;
    if b.len() != n {
        return Err(SolverError::Linalg(format!(
            "b has len {}, need {n}",
            b.len()
        )));
    }
    // Augmented matrix, then forward elimination + back substitution.
    let mut aug = vec![0.0; n * (n + 1)];
    for r in 0..n {
        for c in 0..n {
            aug[r * (n + 1) + c] = a
                .get(r, c)
                .map_err(|e| SolverError::Linalg(e.to_string()))?;
        }
        aug[r * (n + 1) + n] = b[r];
    }
    for col in 0..n {
        let mut pivot = col;
        for r in (col + 1)..n {
            if aug[r * (n + 1) + col].abs() > aug[pivot * (n + 1) + col].abs() {
                pivot = r;
            }
        }
        if aug[pivot * (n + 1) + col] == 0.0 {
            return Err(SolverError::NoSolution("singular system".to_owned()));
        }
        if pivot != col {
            for c in 0..=n {
                aug.swap(col * (n + 1) + c, pivot * (n + 1) + c);
            }
        }
        for r in (col + 1)..n {
            let factor = aug[r * (n + 1) + col] / aug[col * (n + 1) + col];
            for c in col..=n {
                aug[r * (n + 1) + c] -= factor * aug[col * (n + 1) + c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut acc = aug[r * (n + 1) + n];
        for c in (r + 1)..n {
            acc -= aug[r * (n + 1) + c] * x[c];
        }
        let d = aug[r * (n + 1) + r];
        if d == 0.0 {
            return Err(SolverError::NoSolution("singular system".to_owned()));
        }
        x[r] = acc / d;
    }
    Ok(Vector::new(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_2x2() {
        // x + y = 3, x - y = 1 → (2, 1).
        let a = Matrix::from_rows(&[vec![1.0, 1.0], vec![1.0, -1.0]]).unwrap();
        let b = Vector::new(vec![3.0, 1.0]);
        let x = solve_linear_system(&a, &b).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-12);
        assert!((x[1] - 1.0).abs() < 1e-12);
        // Residual check through the matrix layer.
        let back = a.mul_vec(&x).unwrap();
        assert!((back[0] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn singular_errors() {
        let a = Matrix::zeros(2, 2);
        let b = Vector::new(vec![1.0, 2.0]);
        assert!(solve_linear_system(&a, &b).is_err());
        let rect = Matrix::zeros(2, 3);
        assert!(solve_linear_system(&rect, &b).is_err());
    }
}
