//! Dense [`Vector`] of `f64`.

use serde::{Deserialize, Serialize};

use crate::MatrixError;

/// Dense vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    data: Vec<f64>,
}

impl Vector {
    /// New vector (must be non-empty, no NaN).
    #[must_use]
    pub fn new(data: Vec<f64>) -> Self {
        Self { data }
    }

    /// Checked constructor.
    pub fn try_new(data: Vec<f64>) -> Result<Self, MatrixError> {
        if data.is_empty() {
            return Err(MatrixError::InvalidShape(
                "vector must be non-empty".to_owned(),
            ));
        }
        if data.iter().any(|v| v.is_nan()) {
            return Err(MatrixError::InvalidShape(
                "vector entries must not be NaN".to_owned(),
            ));
        }
        Ok(Self { data })
    }

    /// Zero vector of length `n`.
    #[must_use]
    pub fn zeros(n: usize) -> Self {
        Self { data: vec![0.0; n] }
    }

    /// From [`Number`](ocalc_number::Number) slice (via `to_f64`).
    #[must_use]
    pub fn from_numbers(nums: &[ocalc_number::Number]) -> Self {
        Self {
            data: nums.iter().map(|n| n.to_f64()).collect(),
        }
    }

    /// Length (dimension).
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True when empty (only via [`Vector::new`] with empty vec).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Borrowed slice.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    /// Dot product.
    pub fn dot(&self, other: &Self) -> Result<f64, MatrixError> {
        if self.len() != other.len() {
            return Err(MatrixError::DimensionMismatch(format!(
                "dot: len {} vs {}",
                self.len(),
                other.len()
            )));
        }
        Ok(self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum())
    }

    /// Euclidean norm.
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.data.iter().map(|v| v * v).sum::<f64>().sqrt()
    }

    /// Element-wise addition.
    pub fn add(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.len() != other.len() {
            return Err(MatrixError::DimensionMismatch(format!(
                "add: len {} vs {}",
                self.len(),
                other.len()
            )));
        }
        Ok(Self {
            data: self
                .data
                .iter()
                .zip(&other.data)
                .map(|(a, b)| a + b)
                .collect(),
        })
    }

    /// Element-wise subtraction.
    pub fn sub(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.len() != other.len() {
            return Err(MatrixError::DimensionMismatch(format!(
                "sub: len {} vs {}",
                self.len(),
                other.len()
            )));
        }
        Ok(Self {
            data: self
                .data
                .iter()
                .zip(&other.data)
                .map(|(a, b)| a - b)
                .collect(),
        })
    }

    /// Scalar multiplication.
    #[must_use]
    pub fn scale(&self, k: f64) -> Self {
        Self {
            data: self.data.iter().map(|v| v * k).collect(),
        }
    }

    /// 3-D cross product. Errors on other dimensions.
    pub fn cross(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.len() != 3 || other.len() != 3 {
            return Err(MatrixError::DimensionMismatch(
                "cross product requires 3-D vectors".to_owned(),
            ));
        }
        let (a, b) = (&self.data, &other.data);
        Ok(Self {
            data: vec![
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ],
        })
    }
}

impl std::ops::Index<usize> for Vector {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        &self.data[i]
    }
}

impl std::fmt::Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_norm_add_scale() {
        let a = Vector::new(vec![1.0, 2.0, 3.0]);
        let b = Vector::new(vec![4.0, -5.0, 6.0]);
        assert_eq!(a.dot(&b).unwrap(), 12.0);
        assert!((a.norm() - 14f64.sqrt()).abs() < 1e-12);
        assert_eq!(a.add(&b).unwrap().as_slice(), &[5.0, -3.0, 9.0]);
        assert_eq!(a.sub(&b).unwrap().as_slice(), &[-3.0, 7.0, -3.0]);
        assert_eq!(a.scale(2.0).as_slice(), &[2.0, 4.0, 6.0]);
        assert!(a.dot(&Vector::new(vec![1.0])).is_err());
    }

    #[test]
    fn cross() {
        let x = Vector::new(vec![1.0, 0.0, 0.0]);
        let y = Vector::new(vec![0.0, 1.0, 0.0]);
        assert_eq!(x.cross(&y).unwrap().as_slice(), &[0.0, 0.0, 1.0]);
        assert!(x.cross(&Vector::new(vec![1.0, 2.0])).is_err());
    }

    #[test]
    fn checked_constructor() {
        assert!(Vector::try_new(vec![]).is_err());
        assert!(Vector::try_new(vec![f64::NAN]).is_err());
        assert!(Vector::try_new(vec![1.0]).is_ok());
    }
}
