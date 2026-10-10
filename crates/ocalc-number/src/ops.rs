//! Arithmetic operators for [`crate::Number`].

use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use rust_decimal::Decimal;

use crate::{Number, NumberError};

impl Number {
    /// Addition with automatic promotion and overflow handling.
    ///
    /// Fast paths for `Integer + Integer` and `Float + Float` avoid promotion
    /// clones entirely; mixed ranks fall through to [`Self::promote_pair`].
    pub fn add(&self, other: &Self) -> Result<Self, NumberError> {
        // Fast paths: no promotion clones, no heap allocation.
        if let (Self::Integer(x), Self::Integer(y)) = (self, other) {
            return match x.checked_add(*y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(*x) + BigInt::from(*y))),
            };
        }
        if let (Self::Float(x), Self::Float(y)) = (self, other) {
            return Ok(Self::Float(x + y));
        }
        let (a, b) = Self::promote_pair(self, other);
        match (a, b) {
            (Self::Complex(x), Self::Complex(y)) => Ok(Self::Complex(x + y)),
            (Self::Float(x), Self::Float(y)) => Ok(Self::Float(x + y)),
            (Self::Decimal(x), Self::Decimal(y)) => x
                .checked_add(y)
                .map(Self::Decimal)
                .ok_or_else(|| NumberError::Overflow(format!("{x} + {y}"))),
            (Self::Rational(x), Self::Rational(y)) => Ok(Self::from_rational(x + y)),
            (Self::BigInt(x), Self::BigInt(y)) => Ok(Self::from_bigint(x + y)),
            (Self::Integer(x), Self::Integer(y)) => match x.checked_add(y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(x) + BigInt::from(y))),
            },
            _ => unreachable!("promotion guarantees uniform variants"),
        }
    }

    /// Subtraction.
    pub fn sub(&self, other: &Self) -> Result<Self, NumberError> {
        if let (Self::Integer(x), Self::Integer(y)) = (self, other) {
            return match x.checked_sub(*y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(*x) - BigInt::from(*y))),
            };
        }
        if let (Self::Float(x), Self::Float(y)) = (self, other) {
            return Ok(Self::Float(x - y));
        }
        let (a, b) = Self::promote_pair(self, other);
        match (a, b) {
            (Self::Complex(x), Self::Complex(y)) => Ok(Self::Complex(x - y)),
            (Self::Float(x), Self::Float(y)) => Ok(Self::Float(x - y)),
            (Self::Decimal(x), Self::Decimal(y)) => x
                .checked_sub(y)
                .map(Self::Decimal)
                .ok_or_else(|| NumberError::Overflow(format!("{x} - {y}"))),
            (Self::Rational(x), Self::Rational(y)) => Ok(Self::from_rational(x - y)),
            (Self::BigInt(x), Self::BigInt(y)) => Ok(Self::from_bigint(x - y)),
            (Self::Integer(x), Self::Integer(y)) => match x.checked_sub(y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(x) - BigInt::from(y))),
            },
            _ => unreachable!("promotion guarantees uniform variants"),
        }
    }

    /// Multiplication.
    pub fn mul(&self, other: &Self) -> Result<Self, NumberError> {
        if let (Self::Integer(x), Self::Integer(y)) = (self, other) {
            return match x.checked_mul(*y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(*x) * BigInt::from(*y))),
            };
        }
        if let (Self::Float(x), Self::Float(y)) = (self, other) {
            return Ok(Self::Float(x * y));
        }
        let (a, b) = Self::promote_pair(self, other);
        match (a, b) {
            (Self::Complex(x), Self::Complex(y)) => Ok(Self::Complex(x * y)),
            (Self::Float(x), Self::Float(y)) => Ok(Self::Float(x * y)),
            (Self::Decimal(x), Self::Decimal(y)) => x
                .checked_mul(y)
                .map(Self::Decimal)
                .ok_or_else(|| NumberError::Overflow(format!("{x} * {y}"))),
            (Self::Rational(x), Self::Rational(y)) => Ok(Self::from_rational(x * y)),
            (Self::BigInt(x), Self::BigInt(y)) => Ok(Self::from_bigint(x * y)),
            (Self::Integer(x), Self::Integer(y)) => match x.checked_mul(y) {
                Some(v) => Ok(Self::Integer(v)),
                None => Ok(Self::BigInt(BigInt::from(x) * BigInt::from(y))),
            },
            _ => unreachable!("promotion guarantees uniform variants"),
        }
    }

    /// Division. Integer / Rational division by zero errors; float division
    /// by zero also errors (calculator semantics — no silent `inf`).
    pub fn div(&self, other: &Self) -> Result<Self, NumberError> {
        if other.is_zero() {
            return Err(NumberError::DivisionByZero);
        }
        // Fast paths without promotion clones.
        if let (Self::Integer(x), Self::Integer(y)) = (self, other) {
            if x % y == 0 {
                return Ok(Self::Integer(x / y));
            }
            return Ok(Self::Rational(BigRational::new(
                BigInt::from(*x),
                BigInt::from(*y),
            )));
        }
        if let (Self::Float(x), Self::Float(y)) = (self, other) {
            return Ok(Self::Float(x / y));
        }
        let (a, b) = Self::promote_pair(self, other);
        match (a, b) {
            (Self::Complex(x), Self::Complex(y)) => Ok(Self::Complex(x / y)),
            (Self::Float(x), Self::Float(y)) => Ok(Self::Float(x / y)),
            (Self::Decimal(x), Self::Decimal(y)) => x
                .checked_div(y)
                .map(Self::Decimal)
                .ok_or_else(|| NumberError::Overflow(format!("{x} / {y}"))),
            (Self::Rational(x), Self::Rational(y)) => Ok(Self::from_rational(x / y)),
            (Self::BigInt(x), Self::BigInt(y)) => {
                // Exact division when divisible, else rational.
                if (&x % &y).is_zero() {
                    Ok(Self::from_bigint(x / y))
                } else {
                    Ok(Self::Rational(BigRational::new(x, y)))
                }
            }
            (Self::Integer(x), Self::Integer(y)) => {
                if x % y == 0 {
                    // y != 0 checked above; exact.
                    Ok(Self::Integer(x / y))
                } else {
                    Ok(Self::Rational(BigRational::new(
                        BigInt::from(x),
                        BigInt::from(y),
                    )))
                }
            }
            _ => unreachable!("promotion guarantees uniform variants"),
        }
    }

    /// Remainder / modulo. Follows Rust `%` sign-of-dividend semantics for
    /// integers; floats use `rem_euclid`-free `%`.
    pub fn rem(&self, other: &Self) -> Result<Self, NumberError> {
        if other.is_zero() {
            return Err(NumberError::DivisionByZero);
        }
        if let (Self::Integer(x), Self::Integer(y)) = (self, other) {
            return Ok(Self::Integer(
                x.checked_rem(*y).ok_or(NumberError::DivisionByZero)?,
            ));
        }
        if let (Self::Float(x), Self::Float(y)) = (self, other) {
            return Ok(Self::Float(x % y));
        }
        let (a, b) = Self::promote_pair(self, other);
        match (a, b) {
            (Self::Complex(_) | Self::Rational(_), _)
            | (_, Self::Complex(_) | Self::Rational(_)) => Err(NumberError::InvalidOperation(
                "modulo not defined for complex/rational".to_owned(),
            )),
            (Self::Float(x), Self::Float(y)) => Ok(Self::Float(x % y)),
            (Self::Decimal(x), Self::Decimal(y)) => x
                .checked_rem(y)
                .map(Self::Decimal)
                .ok_or_else(|| NumberError::Overflow(format!("{x} % {y}"))),
            (Self::BigInt(x), Self::BigInt(y)) => Ok(Self::from_bigint(x % y)),
            (Self::Integer(x), Self::Integer(y)) => Ok(Self::Integer(
                x.checked_rem(y).ok_or(NumberError::DivisionByZero)?,
            )),
            _ => unreachable!("promotion guarantees uniform variants"),
        }
    }

    /// Unary negation.
    #[must_use]
    pub fn neg(&self) -> Self {
        match self {
            Self::Integer(v) => v
                .checked_neg()
                .map_or(Self::BigInt(-BigInt::from(*v)), Self::Integer),
            Self::BigInt(v) => Self::from_bigint(-v.clone()),
            Self::Rational(v) => Self::Rational(-v.clone()),
            Self::Decimal(v) => Self::Decimal(-v),
            Self::Float(v) => Self::Float(-v),
            Self::Complex(v) => Self::Complex(-v),
        }
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        match self {
            Self::Integer(v) => v
                .checked_abs()
                .map_or(Self::BigInt(BigInt::from(*v).abs()), Self::Integer),
            Self::BigInt(v) => Self::from_bigint(v.abs()),
            Self::Rational(v) => Self::Rational(v.abs()),
            Self::Decimal(v) => Self::Decimal(v.abs()),
            Self::Float(v) => Self::Float(v.abs()),
            Self::Complex(v) => Self::Float(v.norm()),
        }
    }

    /// Power: `self ^ exp`.
    ///
    /// - Integer exponents use exact repeated multiplication (negative
    ///   integer exponents produce a `Rational`).
    /// - Non-integer exponents go through f64 (or complex when needed).
    pub fn pow(&self, exp: &Self) -> Result<Self, NumberError> {
        // Complex fast path.
        if matches!(self, Self::Complex(_)) || matches!(exp, Self::Complex(_)) {
            let b = self.to_complex();
            let e = exp.to_complex();
            return Ok(Self::Complex(b.powc(e)));
        }
        // Integer exponent exact path.
        if let Ok(e) = exp.as_bigint() {
            return self.pow_bigint(&e);
        }
        // Float exponent path.
        let b = self.to_f64();
        let e = exp.to_f64();
        if b < 0.0 && e.fract() != 0.0 {
            // Negative base with fractional exponent → complex.
            let c = Complex64::new(b, 0.0);
            let ce = Complex64::new(e, 0.0);
            return Ok(Self::Complex(c.powc(ce)));
        }
        let r = b.powf(e);
        if r.is_nan() {
            return Err(NumberError::Pow(format!("{self} ^ {exp} is undefined")));
        }
        Ok(Self::Float(r))
    }

    fn pow_bigint(&self, exp: &BigInt) -> Result<Self, NumberError> {
        use num_bigint::Sign;
        let neg = exp.sign() == Sign::Minus;
        let magnitude: u32 = exp
            .abs()
            .to_u32()
            .ok_or_else(|| NumberError::Pow("exponent too large".to_owned()))?;
        // Guard against absurd loops (10^100_000_000 would hang).
        if magnitude > 10_000 {
            // Fall back to float pow for huge exponents.
            let r = self.to_f64().powf(exp.to_f64().unwrap_or(f64::NAN));
            if !r.is_finite() {
                return Err(NumberError::Overflow(format!("{self} ^ {exp}")));
            }
            return Ok(Self::Float(r));
        }
        let mut acc = Self::one_int();
        let base = self.clone();
        for _ in 0..magnitude {
            acc = acc.mul(&base)?;
        }
        if neg {
            Self::one_int().div(&acc)
        } else {
            Ok(acc)
        }
    }

    /// Factorial for non-negative integers (0! = 1). Uses `BigInt` promotion
    /// so `25!` and beyond work exactly.
    pub fn factorial(&self) -> Result<Self, NumberError> {
        let n = self
            .as_bigint()
            .map_err(|_| NumberError::FactorialDomain(self.to_string()))?;
        if n < BigInt::zero() {
            return Err(NumberError::FactorialDomain(self.to_string()));
        }
        let limit = n
            .to_u64()
            .ok_or_else(|| NumberError::Overflow(format!("factorial argument too large: {n}")))?;
        if limit > 10_000 {
            return Err(NumberError::Overflow(format!(
                "factorial argument too large: {limit}"
            )));
        }
        let mut acc = BigInt::from(1);
        for i in 2..=limit {
            acc *= BigInt::from(i);
        }
        Ok(Self::from_bigint(acc))
    }

    /// Square root. Negative reals produce complex results.
    #[must_use]
    pub fn sqrt(&self) -> Self {
        match self {
            Self::Complex(c) => Self::Complex(c.sqrt()),
            Self::Float(f) => {
                if *f < 0.0 {
                    Self::Complex(Complex64::new(*f, 0.0).sqrt())
                } else {
                    Self::Float(f.sqrt())
                }
            }
            Self::Integer(i) if *i < 0 => Self::Complex(Complex64::new(*i as f64, 0.0).sqrt()),
            other => {
                let f = other.to_f64();
                if f < 0.0 {
                    Self::Complex(Complex64::new(f, 0.0).sqrt())
                } else {
                    Self::Float(f.sqrt())
                }
            }
        }
    }

    /// Natural exponential helper (used by function layer too).
    #[must_use]
    pub fn exp(&self) -> Self {
        match self {
            Self::Complex(c) => Self::Complex(c.exp()),
            _ => Self::Float(self.to_f64().exp()),
        }
    }

    /// Natural logarithm helper; non-positive reals → complex.
    pub fn ln(&self) -> Result<Self, NumberError> {
        match self {
            Self::Complex(c) => Ok(Self::Complex(c.ln())),
            _ => {
                let f = self.to_f64();
                if f > 0.0 {
                    Ok(Self::Float(f.ln()))
                } else if f == 0.0 {
                    Err(NumberError::InvalidOperation(
                        "ln(0) is undefined".to_owned(),
                    ))
                } else {
                    Ok(Self::Complex(Complex64::new(f, 0.0).ln()))
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Bitwise (integer-only)
    // ------------------------------------------------------------------

    fn bitwise_pair(&self, other: &Self) -> Result<(BigInt, BigInt), NumberError> {
        Ok((self.as_bigint()?, other.as_bigint()?))
    }

    /// Bitwise AND (integers only).
    pub fn bitand(&self, other: &Self) -> Result<Self, NumberError> {
        let (a, b) = self.bitwise_pair(other)?;
        Ok(Self::from_bigint(a & b))
    }

    /// Bitwise OR (integers only).
    pub fn bitor(&self, other: &Self) -> Result<Self, NumberError> {
        let (a, b) = self.bitwise_pair(other)?;
        Ok(Self::from_bigint(a | b))
    }

    /// Bitwise XOR (integers only).
    pub fn bitxor(&self, other: &Self) -> Result<Self, NumberError> {
        let (a, b) = self.bitwise_pair(other)?;
        Ok(Self::from_bigint(a ^ b))
    }

    /// Bitwise NOT (integers only).
    pub fn bitnot(&self) -> Result<Self, NumberError> {
        Ok(Self::from_bigint(!self.as_bigint()?))
    }

    /// Left shift (`self << n`). `n` must be a non-negative integer.
    pub fn shl(&self, n: &Self) -> Result<Self, NumberError> {
        let v = self.as_bigint()?;
        let s = n.as_bigint()?;
        if s < BigInt::zero() {
            return Err(NumberError::NegativeShift(s.to_string()));
        }
        let shift: usize = s
            .to_usize()
            .ok_or_else(|| NumberError::Overflow(format!("shift amount too large: {s}")))?;
        if shift > 1_000_000 {
            return Err(NumberError::Overflow(format!(
                "shift amount too large: {shift}"
            )));
        }
        Ok(Self::from_bigint(v << shift))
    }

    /// Right shift (`self >> n`).
    pub fn shr(&self, n: &Self) -> Result<Self, NumberError> {
        let v = self.as_bigint()?;
        let s = n.as_bigint()?;
        if s < BigInt::zero() {
            return Err(NumberError::NegativeShift(s.to_string()));
        }
        let shift: usize = s
            .to_usize()
            .ok_or_else(|| NumberError::Overflow(format!("shift amount too large: {s}")))?;
        if shift > 1_000_000 {
            return Err(NumberError::Overflow(format!(
                "shift amount too large: {shift}"
            )));
        }
        Ok(Self::from_bigint(v >> shift))
    }

    // ------------------------------------------------------------------
    // Decimal helpers re-exported for the function layer
    // ------------------------------------------------------------------

    /// Attempt checked add on decimals (used to surface overflow distinctly).
    pub fn checked_add_decimal(a: Decimal, b: Decimal) -> Option<Decimal> {
        a.checked_add(b)
    }
}

impl std::ops::Add for Number {
    type Output = Result<Self, NumberError>;
    fn add(self, rhs: Self) -> Self::Output {
        Number::add(&self, &rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn int_overflow_promotes_to_bigint() {
        let a = Number::Integer(i64::MAX);
        let b = Number::Integer(1);
        let r = a.add(&b).unwrap();
        assert!(matches!(r, Number::BigInt(_)));
        assert_eq!(r.to_string(), "9223372036854775808");
    }

    #[test]
    fn int_div_produces_rational_when_not_exact() {
        let r = Number::Integer(1).div(&Number::Integer(3)).unwrap();
        assert!(matches!(r, Number::Rational(_)));
        assert_eq!(r.to_f64(), 1.0 / 3.0);
    }

    #[test]
    fn div_by_zero_errors() {
        for zero in [
            Number::Integer(0),
            Number::Float(0.0),
            Number::from_bigint(BigInt::zero()),
        ] {
            assert_eq!(
                Number::Integer(1).div(&zero).unwrap_err(),
                NumberError::DivisionByZero
            );
        }
    }

    #[test]
    fn factorial_values() {
        assert_eq!(Number::Integer(0).factorial().unwrap(), Number::Integer(1));
        assert_eq!(
            Number::Integer(5).factorial().unwrap(),
            Number::Integer(120)
        );
        assert_eq!(
            Number::Integer(25).factorial().unwrap().to_string(),
            "15511210043330985984000000"
        );
        assert!(Number::Integer(-1).factorial().is_err());
        assert!(Number::Float(2.5).factorial().is_err());
    }

    #[test]
    fn bitwise_requires_integers() {
        assert_eq!(
            Number::Integer(6).bitand(&Number::Integer(3)).unwrap(),
            Number::Integer(2)
        );
        assert_eq!(
            Number::Integer(6).bitor(&Number::Integer(3)).unwrap(),
            Number::Integer(7)
        );
        assert_eq!(
            Number::Integer(6).bitxor(&Number::Integer(3)).unwrap(),
            Number::Integer(5)
        );
        assert!(Number::Float(1.5).bitand(&Number::Integer(1)).is_err());
    }

    #[test]
    fn shifts() {
        assert_eq!(
            Number::Integer(1).shl(&Number::Integer(8)).unwrap(),
            Number::Integer(256)
        );
        assert_eq!(
            Number::Integer(256).shr(&Number::Integer(8)).unwrap(),
            Number::Integer(1)
        );
        assert!(Number::Integer(1).shl(&Number::Integer(-1)).is_err());
    }

    #[test]
    fn pow_integer_exact_and_negative() {
        assert_eq!(
            Number::Integer(2).pow(&Number::Integer(10)).unwrap(),
            Number::Integer(1024)
        );
        let r = Number::Integer(2).pow(&Number::Integer(-1)).unwrap();
        assert_eq!(r.to_f64(), 0.5);
    }

    #[test]
    fn sqrt_negative_gives_complex() {
        let r = Number::Integer(-4).sqrt();
        assert!(matches!(r, Number::Complex(_)));
    }

    #[test]
    fn decimal_fromstr() {
        let n = Number::from_str("3.50").unwrap();
        assert!(matches!(n, Number::Decimal(_)));
    }

    #[test]
    fn rational_arithmetic_stays_exact() {
        let half = Number::Integer(1).div(&Number::Integer(2)).unwrap();
        let third = Number::Integer(1).div(&Number::Integer(3)).unwrap();
        // 1/2 + 1/3 = 5/6 exactly.
        let s = half.add(&third).unwrap();
        assert_eq!(s.to_string(), "5/6");
        // (1/3) * 3 == 1 exactly.
        let one = third.mul(&Number::Integer(3)).unwrap();
        assert_eq!(one, Number::Integer(1));
    }

    #[test]
    fn complex_arithmetic() {
        let a = Number::complex(1.0, 2.0);
        let b = Number::complex(3.0, -4.0);
        // (1+2i) + (3-4i) = 4-2i
        assert_eq!(a.add(&b).unwrap().to_string(), "4 - 2i");
        // (1+2i) * (3-4i) = 11+2i
        assert_eq!(a.mul(&b).unwrap().to_string(), "11 + 2i");
        // |3+4i| = 5
        assert_eq!(Number::complex(3.0, 4.0).abs(), Number::Float(5.0));
        // sqrt(-9) = 3i
        assert_eq!(Number::Integer(-9).sqrt().to_string(), "3i");
    }

    #[test]
    fn mixed_promotion_prefers_higher_rank() {
        // Integer + Decimal → Decimal.
        let r = Number::Integer(1)
            .add(&Number::from_decimal_str("2.5").unwrap())
            .unwrap();
        assert!(matches!(r, Number::Decimal(_)));
        // Anything + Complex → Complex.
        let c = Number::Integer(1).add(&Number::complex(0.0, 1.0)).unwrap();
        assert!(matches!(c, Number::Complex(_)));
        // Integer + Float → Float.
        let f = Number::Integer(1).add(&Number::Float(0.5)).unwrap();
        assert!(matches!(f, Number::Float(_)));
    }
}
