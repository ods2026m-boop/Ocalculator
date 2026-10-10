//! Dense row-major [`Matrix`] of `f64`.
//!
//! Foundation for linear algebra in Ocalculator: construction, basic
//! arithmetic, transpose, determinant and inverse. Exact [`Number`](ocalc_number::Number)
//! scalars convert via [`Matrix::from_numbers`] (through `to_f64`); future
//! work can add an exact rational matrix on top without changing this API.

use serde::{Deserialize, Serialize};

use crate::MatrixError;
use crate::Vector;

/// Dense row-major matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f64>,
}

impl Matrix {
    /// Build from raw row-major data. Errors on empty dims or length mismatch.
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Result<Self, MatrixError> {
        if rows == 0 || cols == 0 {
            return Err(MatrixError::InvalidShape(format!(
                "{rows}x{cols} must be non-empty"
            )));
        }
        if data.len() != rows * cols {
            return Err(MatrixError::InvalidShape(format!(
                "expected {} values for {rows}x{cols}, got {}",
                rows * cols,
                data.len()
            )));
        }
        if data.iter().any(|v| v.is_nan()) {
            return Err(MatrixError::InvalidShape(
                "matrix entries must not be NaN".to_owned(),
            ));
        }
        Ok(Self { rows, cols, data })
    }

    /// Build from row vectors. Errors on ragged/empty input.
    pub fn from_rows(rows: &[Vec<f64>]) -> Result<Self, MatrixError> {
        if rows.is_empty() || rows[0].is_empty() {
            return Err(MatrixError::InvalidShape(
                "rows must be non-empty".to_owned(),
            ));
        }
        let cols = rows[0].len();
        if rows.iter().any(|r| r.len() != cols) {
            return Err(MatrixError::InvalidShape("ragged rows".to_owned()));
        }
        let data = rows.iter().flatten().copied().collect();
        Self::new(rows.len(), cols, data)
    }

    /// Build from [`Number`](ocalc_number::Number) rows (via `to_f64`).
    pub fn from_numbers(rows: &[Vec<ocalc_number::Number>]) -> Result<Self, MatrixError> {
        let floats: Vec<Vec<f64>> = rows
            .iter()
            .map(|r| r.iter().map(|n| n.to_f64()).collect())
            .collect();
        Self::from_rows(&floats)
    }

    /// Zero matrix.
    #[must_use]
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// Identity matrix (square).
    #[must_use]
    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.data[i * n + i] = 1.0;
        }
        m
    }

    /// Row count.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Column count.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Shape as `(rows, cols)`.
    #[must_use]
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// True for square matrices.
    #[must_use]
    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    /// Element at `(row, col)`.
    pub fn get(&self, row: usize, col: usize) -> Result<f64, MatrixError> {
        if row >= self.rows || col >= self.cols {
            return Err(MatrixError::IndexOutOfBounds {
                row,
                col,
                rows: self.rows,
                cols: self.cols,
            });
        }
        Ok(self.data[row * self.cols + col])
    }

    /// Set element at `(row, col)`.
    pub fn set(&mut self, row: usize, col: usize, value: f64) -> Result<(), MatrixError> {
        if row >= self.rows || col >= self.cols {
            return Err(MatrixError::IndexOutOfBounds {
                row,
                col,
                rows: self.rows,
                cols: self.cols,
            });
        }
        self.data[row * self.cols + col] = value;
        Ok(())
    }

    /// Row-major data slice.
    #[must_use]
    pub fn data(&self) -> &[f64] {
        &self.data
    }

    /// Transpose.
    #[must_use]
    pub fn transpose(&self) -> Self {
        let mut out = vec![0.0; self.rows * self.cols];
        for r in 0..self.rows {
            for c in 0..self.cols {
                out[c * self.rows + r] = self.data[r * self.cols + c];
            }
        }
        Self {
            rows: self.cols,
            cols: self.rows,
            data: out,
        }
    }

    /// Element-wise addition.
    pub fn add(&self, other: &Self) -> Result<Self, MatrixError> {
        self.check_same_shape(other, "add")?;
        let data = self
            .data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| a + b)
            .collect();
        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    /// Element-wise subtraction.
    pub fn sub(&self, other: &Self) -> Result<Self, MatrixError> {
        self.check_same_shape(other, "sub")?;
        let data = self
            .data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| a - b)
            .collect();
        Ok(Self {
            rows: self.rows,
            cols: self.cols,
            data,
        })
    }

    /// Scalar multiplication.
    #[must_use]
    pub fn scale(&self, k: f64) -> Self {
        let data = self.data.iter().map(|v| v * k).collect();
        Self {
            rows: self.rows,
            cols: self.cols,
            data,
        }
    }

    /// Matrix product `self * other`.
    pub fn mul(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.cols != other.rows {
            return Err(MatrixError::DimensionMismatch(format!(
                "cannot multiply {}x{} by {}x{}",
                self.rows, self.cols, other.rows, other.cols
            )));
        }
        let mut out = vec![0.0; self.rows * other.cols];
        for r in 0..self.rows {
            for k in 0..self.cols {
                let aik = self.data[r * self.cols + k];
                for c in 0..other.cols {
                    out[r * other.cols + c] += aik * other.data[k * other.cols + c];
                }
            }
        }
        Ok(Self {
            rows: self.rows,
            cols: other.cols,
            data: out,
        })
    }

    /// Matrix–vector product.
    pub fn mul_vec(&self, v: &Vector) -> Result<Vector, MatrixError> {
        if self.cols != v.len() {
            return Err(MatrixError::DimensionMismatch(format!(
                "cannot multiply {}x{} by vector of len {}",
                self.rows,
                self.cols,
                v.len()
            )));
        }
        let out = self
            .data
            .chunks_exact(self.cols)
            .map(|row| row.iter().zip(v.as_slice()).map(|(a, b)| a * b).sum())
            .collect();
        Ok(Vector::new(out))
    }

    /// Determinant via Gaussian elimination with partial pivoting.
    /// 1x1/2x2 use direct formulas.
    pub fn determinant(&self) -> Result<f64, MatrixError> {
        if !self.is_square() {
            return Err(MatrixError::NotSquare {
                rows: self.rows,
                cols: self.cols,
            });
        }
        match self.rows {
            1 => Ok(self.data[0]),
            2 => Ok(self.data[0] * self.data[3] - self.data[1] * self.data[2]),
            n => {
                let mut a = self.data.clone();
                let mut det = 1.0;
                for col in 0..n {
                    // Partial pivot.
                    let mut pivot = col;
                    for r in (col + 1)..n {
                        if a[r * n + col].abs() > a[pivot * n + col].abs() {
                            pivot = r;
                        }
                    }
                    if a[pivot * n + col] == 0.0 {
                        return Ok(0.0);
                    }
                    if pivot != col {
                        for c in 0..n {
                            a.swap(col * n + c, pivot * n + c);
                        }
                        det = -det;
                    }
                    let pv = a[col * n + col];
                    det *= pv;
                    for r in (col + 1)..n {
                        let factor = a[r * n + col] / pv;
                        for c in col..n {
                            a[r * n + c] -= factor * a[col * n + c];
                        }
                    }
                }
                Ok(det)
            }
        }
    }

    /// Inverse via Gauss-Jordan. Errors [`MatrixError::Singular`].
    pub fn inverse(&self) -> Result<Self, MatrixError> {
        if !self.is_square() {
            return Err(MatrixError::NotSquare {
                rows: self.rows,
                cols: self.cols,
            });
        }
        let n = self.rows;
        if n == 1 {
            if self.data[0] == 0.0 {
                return Err(MatrixError::Singular);
            }
            return Ok(Self {
                rows: 1,
                cols: 1,
                data: vec![1.0 / self.data[0]],
            });
        }
        if n == 2 {
            let det = self.determinant()?;
            if det == 0.0 {
                return Err(MatrixError::Singular);
            }
            let (a, b, c, d) = (self.data[0], self.data[1], self.data[2], self.data[3]);
            return Ok(Self {
                rows: 2,
                cols: 2,
                data: vec![d / det, -b / det, -c / det, a / det],
            });
        }
        // Augmented [A | I], row-reduce to [I | A^-1].
        let mut aug = vec![0.0; n * 2 * n];
        for r in 0..n {
            for c in 0..n {
                aug[r * 2 * n + c] = self.data[r * n + c];
            }
            aug[r * 2 * n + n + r] = 1.0;
        }
        for col in 0..n {
            let mut pivot = col;
            for r in (col + 1)..n {
                if aug[r * 2 * n + col].abs() > aug[pivot * 2 * n + col].abs() {
                    pivot = r;
                }
            }
            if aug[pivot * 2 * n + col] == 0.0 {
                return Err(MatrixError::Singular);
            }
            if pivot != col {
                for c in 0..2 * n {
                    aug.swap(col * 2 * n + c, pivot * 2 * n + c);
                }
            }
            let pv = aug[col * 2 * n + col];
            for c in 0..2 * n {
                aug[col * 2 * n + c] /= pv;
            }
            for r in 0..n {
                if r != col {
                    let factor = aug[r * 2 * n + col];
                    for c in 0..2 * n {
                        aug[r * 2 * n + c] -= factor * aug[col * 2 * n + c];
                    }
                }
            }
        }
        let mut data = vec![0.0; n * n];
        for r in 0..n {
            for c in 0..n {
                data[r * n + c] = aug[r * 2 * n + n + c];
            }
        }
        Ok(Self {
            rows: n,
            cols: n,
            data,
        })
    }

    fn check_same_shape(&self, other: &Self, op: &str) -> Result<(), MatrixError> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(MatrixError::DimensionMismatch(format!(
                "cannot {op} {}x{} with {}x{}",
                self.rows, self.cols, other.rows, other.cols
            )));
        }
        Ok(())
    }
}

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for r in 0..self.rows {
            write!(f, "[")?;
            for c in 0..self.cols {
                if c > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.data[r * self.cols + c])?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m2() -> Matrix {
        Matrix::from_rows(&[vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap()
    }

    #[test]
    fn construction_validates() {
        assert!(Matrix::new(0, 2, vec![]).is_err());
        assert!(Matrix::new(2, 2, vec![1.0, 2.0]).is_err());
        assert!(Matrix::from_rows(&[vec![1.0], vec![1.0, 2.0]]).is_err());
        assert!(Matrix::from_rows(&[] as &[Vec<f64>]).is_err());
    }

    #[test]
    fn add_sub_scale() {
        let a = m2();
        let b = Matrix::identity(2);
        let s = a.add(&b).unwrap();
        assert_eq!(s.get(0, 0).unwrap(), 2.0);
        assert_eq!(a.sub(&b).unwrap().get(1, 1).unwrap(), 3.0);
        assert_eq!(a.scale(2.0).get(0, 1).unwrap(), 4.0);
        assert!(a.add(&Matrix::zeros(2, 3)).is_err());
    }

    #[test]
    fn mul_and_mat_vec() {
        let a = m2();
        let b = a.mul(&a).unwrap();
        // [[1,2],[3,4]]^2 = [[7,10],[15,22]]
        assert_eq!(b.data(), &[7.0, 10.0, 15.0, 22.0]);
        let v = a.mul_vec(&Vector::new(vec![1.0, 1.0])).unwrap();
        assert_eq!(v.as_slice(), &[3.0, 7.0]);
        assert!(a.mul(&Matrix::zeros(3, 3)).is_err());
    }

    #[test]
    fn transpose() {
        let a = Matrix::from_rows(&[vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        let t = a.transpose();
        assert_eq!(t.shape(), (3, 2));
        assert_eq!(t.get(2, 0).unwrap(), 3.0);
        assert_eq!(a.transpose().transpose(), a);
    }

    #[test]
    fn determinant() {
        assert_eq!(Matrix::identity(3).determinant().unwrap(), 1.0);
        assert_eq!(m2().determinant().unwrap(), -2.0);
        let m3 = Matrix::from_rows(&[
            vec![6.0, 1.0, 1.0],
            vec![4.0, -2.0, 5.0],
            vec![2.0, 8.0, 7.0],
        ])
        .unwrap();
        assert!((m3.determinant().unwrap() + 306.0).abs() < 1e-9);
        assert_eq!(Matrix::zeros(2, 2).determinant().unwrap(), 0.0);
        assert!(Matrix::zeros(2, 3).determinant().is_err());
    }

    #[test]
    fn inverse() {
        let inv = m2().inverse().unwrap();
        let id = m2().mul(&inv).unwrap();
        for (got, want) in id.data().iter().zip(Matrix::identity(2).data()) {
            assert!((got - want).abs() < 1e-12);
        }
        assert!(Matrix::zeros(2, 2).inverse().is_err());
        assert!(Matrix::zeros(2, 3).inverse().is_err());
    }

    #[test]
    fn from_numbers() {
        use ocalc_number::Number;
        let m = Matrix::from_numbers(&[
            vec![Number::Integer(1), Number::Integer(2)],
            vec![Number::Integer(3), Number::Integer(4)],
        ])
        .unwrap();
        assert_eq!(m, m2());
    }
}
