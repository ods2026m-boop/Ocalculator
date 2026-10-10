//! The [`Number`] type: construction, conversion, inspection.

use std::fmt;
use std::str::FromStr;

use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::NumberError;

/// Unified numeric value used across the whole Ocalculator ecosystem.
///
/// Cloning is cheap for small variants; `BigInt`/`BigRational` clone their
/// heap. For hot evaluator paths prefer the `Integer`/`Float` fast paths.
///
/// Serialized as a version-stable tagged object
/// (`{"kind": "...", "value": ...}`) so history files and saved expressions
/// round-trip exactly (including `BigInt`/`Rational`/`Decimal` precision and
/// non-finite floats). The `kind` tag is part of the Phase 3 stable format
/// (v1) and must not be renamed.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// Fast-path 64-bit signed integer.
    Integer(i64),
    /// Arbitrary precision integer.
    BigInt(BigInt),
    /// Exact rational (`BigRational`).
    Rational(BigRational),
    /// Base-10 decimal with up to 28-29 significant digits.
    Decimal(Decimal),
    /// IEEE-754 binary64.
    Float(f64),
    /// Complex with f64 components.
    Complex(Complex64),
}

impl Number {
    // ------------------------------------------------------------------
    // Constructors
    // ------------------------------------------------------------------

    /// Zero as an integer.
    #[inline]
    #[must_use]
    pub fn zero_int() -> Self {
        Self::Integer(0)
    }

    /// One as an integer.
    #[inline]
    #[must_use]
    pub fn one_int() -> Self {
        Self::Integer(1)
    }

    /// Construct from f64, mapping NaN/∞ to errors is the caller's job;
    /// non-finite floats are preserved but flagged by [`Number::is_finite`].
    #[inline]
    #[must_use]
    pub fn from_f64(v: f64) -> Self {
        Self::Float(v)
    }

    /// Construct from i64.
    #[inline]
    #[must_use]
    pub fn from_i64(v: i64) -> Self {
        Self::Integer(v)
    }

    /// Construct from BigInt, normalising values that fit in i64.
    #[must_use]
    pub fn from_bigint(v: BigInt) -> Self {
        if let Some(small) = v.to_i64() {
            Self::Integer(small)
        } else {
            Self::BigInt(v)
        }
    }

    /// Construct from an exact rational, normalising integers.
    #[must_use]
    pub fn from_rational(v: BigRational) -> Self {
        if v.is_integer() {
            Self::from_bigint(v.to_integer())
        } else {
            Self::Rational(v)
        }
    }

    /// Construct a rational from a numerator/denominator pair.
    ///
    /// Returns `None` when `den` is zero.
    #[must_use]
    pub fn rational(num: BigInt, den: BigInt) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        Some(Self::from_rational(BigRational::new(num, den)))
    }

    /// Construct a complex value; a zero imaginary part normalises to [`Self::Float`].
    #[must_use]
    pub fn complex(re: f64, im: f64) -> Self {
        Self::from(Complex64::new(re, im))
    }

    /// Short type name (`"integer"`, `"bigint"`, `"rational"`, `"decimal"`,
    /// `"float"`, `"complex"`) for diagnostics and conversions.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Integer(_) => "integer",
            Self::BigInt(_) => "bigint",
            Self::Rational(_) => "rational",
            Self::Decimal(_) => "decimal",
            Self::Float(_) => "float",
            Self::Complex(_) => "complex",
        }
    }

    /// True for [`Self::Complex`] with a non-zero imaginary part.
    #[must_use]
    pub fn is_complex(&self) -> bool {
        matches!(self, Self::Complex(c) if c.im != 0.0)
    }

    /// Lossy extraction to `i64` (complex → real part truncated toward zero).
    /// Returns `None` for out-of-range, non-finite or non-integer values.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            Self::BigInt(v) => v.to_i64(),
            Self::Rational(r) if r.is_integer() => r.to_integer().to_i64(),
            Self::Decimal(d) if d.fract().is_zero() => d.to_i64(),
            Self::Float(f) if f.fract() == 0.0 && f.is_finite() => {
                if *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                    Some(*f as i64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Parse a decimal integer literal in the given radix (2, 8, 10, 16).
    ///
    /// Handles optional leading `+`/`-` and `_` separators.
    pub fn from_int_str(s: &str, radix: u32) -> Result<Self, NumberError> {
        let clean: String = s.chars().filter(|&c| c != '_').collect();
        let (neg, digits) = match clean.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, clean.strip_prefix('+').unwrap_or(&clean)),
        };
        let digits = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
            .or_else(|| {
                digits
                    .strip_prefix("0o")
                    .or_else(|| digits.strip_prefix("0O"))
            })
            .or_else(|| {
                digits
                    .strip_prefix("0b")
                    .or_else(|| digits.strip_prefix("0B"))
            })
            .unwrap_or(digits);
        if digits.is_empty() {
            return Err(NumberError::Parse(s.to_owned()));
        }
        match BigInt::parse_bytes(digits.as_bytes(), radix) {
            Some(mut v) => {
                if neg {
                    v = -v;
                }
                Ok(Self::from_bigint(v))
            }
            None => Err(NumberError::Parse(s.to_owned())),
        }
    }

    /// Parse a base-10 float/decimal literal (supports `e`/`E` exponents).
    ///
    /// Strategy: if the literal contains `.`, `e` or `E`, first try an exact
    /// `Decimal`; on failure fall back to f64. Pure integers become `Integer`.
    pub fn from_decimal_str(s: &str) -> Result<Self, NumberError> {
        let clean: String = s.chars().filter(|&c| c != '_').collect();
        if clean.contains('.') || clean.contains('e') || clean.contains('E') {
            if let Ok(d) = Decimal::from_str(&clean) {
                // Normalise: if it is integral and fits, keep as Integer?
                // Keep as Decimal to preserve user intent (decimal literal).
                return Ok(Self::Decimal(d));
            }
            match clean.parse::<f64>() {
                Ok(f) => Ok(Self::Float(f)),
                Err(_) => Err(NumberError::Parse(s.to_owned())),
            }
        } else {
            Self::from_int_str(&clean, 10)
        }
    }

    // ------------------------------------------------------------------
    // Inspection
    // ------------------------------------------------------------------

    /// Rank used for type promotion (higher wins).
    fn rank(&self) -> u8 {
        match self {
            Self::Integer(_) => 0,
            Self::BigInt(_) => 1,
            Self::Rational(_) => 2,
            Self::Decimal(_) => 3,
            Self::Float(_) => 4,
            Self::Complex(_) => 5,
        }
    }

    /// True if the value is numerically zero (any variant).
    #[must_use]
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Integer(v) => *v == 0,
            Self::BigInt(v) => v.is_zero(),
            Self::Rational(v) => v.is_zero(),
            Self::Decimal(v) => v.is_zero(),
            Self::Float(v) => *v == 0.0,
            Self::Complex(v) => v.norm() == 0.0,
        }
    }

    /// True if the value is a mathematical integer (including `3.0` floats,
    /// integral decimals/rationals).
    #[must_use]
    pub fn is_integer_valued(&self) -> bool {
        match self {
            Self::Integer(_) | Self::BigInt(_) => true,
            Self::Rational(r) => r.is_integer(),
            Self::Decimal(d) => d.fract().is_zero(),
            Self::Float(f) => f.fract() == 0.0 && f.is_finite(),
            Self::Complex(c) => c.im == 0.0 && c.re.fract() == 0.0 && c.re.is_finite(),
        }
    }

    /// True for finite floats (integers/decimals/rationals are always finite
    /// unless they overflowed — which they can't by construction).
    #[must_use]
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Float(f) => f.is_finite(),
            Self::Complex(c) => c.re.is_finite() && c.im.is_finite(),
            _ => true,
        }
    }

    /// True if negative (complex: real part < 0 with zero imaginary is
    /// considered negative; general complex returns real-part sign).
    #[must_use]
    pub fn is_negative(&self) -> bool {
        match self {
            Self::Integer(v) => *v < 0,
            Self::BigInt(v) => v.sign() == num_bigint::Sign::Minus,
            Self::Rational(v) => v.numer().sign() == num_bigint::Sign::Minus,
            Self::Decimal(v) => v.is_sign_negative(),
            Self::Float(v) => *v < 0.0,
            Self::Complex(v) => v.re < 0.0,
        }
    }

    // ------------------------------------------------------------------
    // Conversions
    // ------------------------------------------------------------------

    /// Lossy conversion to f64 (complex → magnitude).
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        match self {
            Self::Integer(v) => *v as f64,
            Self::BigInt(v) => v.to_f64().unwrap_or(f64::NAN),
            Self::Rational(v) => v.to_f64().unwrap_or(f64::NAN),
            Self::Decimal(v) => v.to_f64().unwrap_or(f64::NAN),
            Self::Float(v) => *v,
            Self::Complex(v) => v.norm(),
        }
    }

    /// Convert to `Complex64`, if representable.
    #[must_use]
    pub fn to_complex(&self) -> Complex64 {
        match self {
            Self::Integer(v) => Complex64::new(*v as f64, 0.0),
            Self::BigInt(v) => Complex64::new(v.to_f64().unwrap_or(f64::NAN), 0.0),
            Self::Rational(v) => Complex64::new(v.to_f64().unwrap_or(f64::NAN), 0.0),
            Self::Decimal(v) => Complex64::new(v.to_f64().unwrap_or(f64::NAN), 0.0),
            Self::Float(v) => Complex64::new(*v, 0.0),
            Self::Complex(v) => *v,
        }
    }

    /// Convert to `BigRational` exactly when possible.
    /// Floats that are non-finite return `None`.
    #[must_use]
    pub fn to_rational(&self) -> Option<BigRational> {
        match self {
            Self::Integer(v) => Some(BigRational::from_integer(BigInt::from(*v))),
            Self::BigInt(v) => Some(BigRational::from_integer(v.clone())),
            Self::Rational(v) => Some(v.clone()),
            Self::Decimal(v) => BigRational::from_float(v.to_f64()?),
            Self::Float(v) => BigRational::from_float(*v),
            Self::Complex(_) => None,
        }
    }

    /// Convert to `Decimal` when exactly representable.
    #[must_use]
    pub fn to_decimal(&self) -> Option<Decimal> {
        match self {
            Self::Decimal(v) => Some(*v),
            Self::Integer(v) => Some(Decimal::from(*v)),
            Self::BigInt(v) => Decimal::from_str(&v.to_string()).ok(),
            Self::Rational(v) => {
                let f = v.to_f64()?;
                Decimal::from_f64(f)
            }
            Self::Float(v) => Decimal::from_f64(*v),
            Self::Complex(_) => None,
        }
    }

    /// Extract as `BigInt` if the value is integer-valued.
    pub fn as_bigint(&self) -> Result<BigInt, NumberError> {
        match self {
            Self::Integer(v) => Ok(BigInt::from(*v)),
            Self::BigInt(v) => Ok(v.clone()),
            Self::Rational(r) => {
                if r.is_integer() {
                    Ok(r.to_integer())
                } else {
                    Err(NumberError::NonIntegerBitwise(self.to_string()))
                }
            }
            Self::Decimal(d) => {
                if d.fract().is_zero() {
                    // `Decimal` has no direct `to_bigint`; go through its
                    // canonical integer string (exact for integral values).
                    BigInt::parse_bytes(d.trunc().to_string().as_bytes(), 10).ok_or_else(|| {
                        NumberError::InvalidOperation(format!("cannot convert {d} to integer"))
                    })
                } else {
                    Err(NumberError::NonIntegerBitwise(self.to_string()))
                }
            }
            Self::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    BigInt::from_f64(*f).ok_or_else(|| {
                        NumberError::InvalidOperation(format!("cannot convert {f} to integer"))
                    })
                } else {
                    Err(NumberError::NonIntegerBitwise(self.to_string()))
                }
            }
            Self::Complex(c) => {
                if c.im == 0.0 && c.re.fract() == 0.0 && c.re.is_finite() {
                    BigInt::from_f64(c.re).ok_or_else(|| {
                        NumberError::InvalidOperation(format!("cannot convert {c} to integer"))
                    })
                } else {
                    Err(NumberError::NonIntegerBitwise(self.to_string()))
                }
            }
        }
    }

    /// Promote both operands to a common representation rank.
    pub(crate) fn promote_pair(a: &Self, b: &Self) -> (Self, Self) {
        let target = a.rank().max(b.rank());
        (a.promote_to(target), b.promote_to(target))
    }

    pub(crate) fn promote_to(&self, rank: u8) -> Self {
        if self.rank() == rank {
            return self.clone();
        }
        match rank {
            5 => Self::Complex(self.to_complex()),
            4 => Self::Float(self.to_f64()),
            3 => Self::Decimal(
                self.to_decimal()
                    .unwrap_or_else(|| Decimal::from_f64(self.to_f64()).unwrap_or(Decimal::ZERO)),
            ),
            // NOTE: `BigInt`/`Rational` targets construct the variant directly:
            // the normalising `from_*` constructors would fold small values
            // back to `Integer` and break `promote_pair`'s uniform-variant
            // invariant (the source of a past `unreachable!` panic).
            2 => Self::Rational(
                self.to_rational()
                    .unwrap_or_else(|| BigRational::from_integer(BigInt::zero())),
            ),
            // NOTE: construct `BigInt` directly (not via `from_bigint`, which
            // would normalise small values back to `Integer` and break the
            // uniform-variant invariant `promote_pair` relies on).
            1 => Self::BigInt(self.as_bigint().unwrap_or_else(|_| BigInt::zero())),
            _ => match self.as_bigint() {
                Ok(b) => match b.to_i64() {
                    Some(i) => Self::Integer(i),
                    None => Self::BigInt(b),
                },
                Err(_) => Self::Integer(0),
            },
        }
    }
}

// ----------------------------------------------------------------------
// Display / parsing
// ----------------------------------------------------------------------

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::BigInt(v) => write!(f, "{v}"),
            Self::Rational(v) => {
                if v.is_integer() {
                    write!(f, "{}", v.to_integer())
                } else {
                    write!(f, "{v}")
                }
            }
            Self::Decimal(v) => write!(f, "{v}"),
            Self::Float(v) => {
                if v.is_nan() {
                    write!(f, "NaN")
                } else if v.is_infinite() {
                    if *v > 0.0 {
                        write!(f, "inf")
                    } else {
                        write!(f, "-inf")
                    }
                } else if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{v:.1}")
                } else {
                    write!(f, "{v}")
                }
            }
            Self::Complex(v) => {
                if v.im == 0.0 {
                    write!(f, "{}", v.re)
                } else if v.re == 0.0 {
                    write!(f, "{}i", v.im)
                } else if v.im < 0.0 {
                    write!(f, "{} - {}i", v.re, v.im.abs())
                } else {
                    write!(f, "{} + {}i", v.re, v.im)
                }
            }
        }
    }
}

impl FromStr for Number {
    type Err = NumberError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        // Complex literal like "3+4i" is handled by the engine, not here.
        if t.ends_with('i') || t.ends_with('j') {
            return Err(NumberError::Parse(s.to_owned()));
        }
        Self::from_decimal_str(t)
    }
}

impl From<i64> for Number {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}

impl From<i32> for Number {
    fn from(v: i32) -> Self {
        Self::Integer(i64::from(v))
    }
}

impl From<f64> for Number {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<BigInt> for Number {
    fn from(v: BigInt) -> Self {
        Self::from_bigint(v)
    }
}

impl From<BigRational> for Number {
    fn from(v: BigRational) -> Self {
        Self::from_rational(v)
    }
}

impl From<u64> for Number {
    fn from(v: u64) -> Self {
        if v <= i64::MAX as u64 {
            Self::Integer(v as i64)
        } else {
            Self::BigInt(BigInt::from(v))
        }
    }
}

impl From<i128> for Number {
    fn from(v: i128) -> Self {
        if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
            Self::Integer(v as i64)
        } else {
            Self::BigInt(BigInt::from(v))
        }
    }
}

impl From<Decimal> for Number {
    fn from(v: Decimal) -> Self {
        Self::Decimal(v)
    }
}

impl From<Complex64> for Number {
    fn from(v: Complex64) -> Self {
        if v.im == 0.0 {
            Self::Float(v.re)
        } else {
            Self::Complex(v)
        }
    }
}

// ----------------------------------------------------------------------
// Stable serde (Phase 3, format v1)
// ----------------------------------------------------------------------

/// Wire shape for [`Number`]. Human-readable JSON uses exact strings for
/// arbitrary-precision variants so no precision is lost.
#[derive(Serialize, Deserialize)]
struct NumberWire {
    kind: String,
    value: serde_json::Value,
}

fn float_to_json(f: f64) -> serde_json::Value {
    if f.is_nan() {
        serde_json::Value::String("NaN".to_owned())
    } else if f.is_infinite() {
        if f > 0.0 {
            serde_json::Value::String("inf".to_owned())
        } else {
            serde_json::Value::String("-inf".to_owned())
        }
    } else {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::String("NaN".to_owned()))
    }
}

fn json_to_float(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => match s.as_str() {
            "NaN" | "nan" => Some(f64::NAN),
            "inf" | "+inf" | "Infinity" => Some(f64::INFINITY),
            "-inf" | "-Infinity" => Some(f64::NEG_INFINITY),
            other => other.parse::<f64>().ok(),
        },
        _ => None,
    }
}

impl Serialize for Number {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = match self {
            Self::Integer(v) => NumberWire {
                kind: "integer".to_owned(),
                value: serde_json::Value::Number(serde_json::Number::from(*v)),
            },
            Self::BigInt(v) => NumberWire {
                kind: "bigint".to_owned(),
                value: serde_json::Value::String(v.to_string()),
            },
            Self::Rational(v) => NumberWire {
                kind: "rational".to_owned(),
                value: serde_json::Value::String(v.to_string()),
            },
            Self::Decimal(v) => NumberWire {
                kind: "decimal".to_owned(),
                value: serde_json::Value::String(v.to_string()),
            },
            Self::Float(v) => NumberWire {
                kind: "float".to_owned(),
                value: float_to_json(*v),
            },
            Self::Complex(v) => NumberWire {
                kind: "complex".to_owned(),
                value: serde_json::json!({
                    "re": float_to_json(v.re),
                    "im": float_to_json(v.im),
                }),
            },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let wire = NumberWire::deserialize(deserializer)?;
        match wire.kind.as_str() {
            "integer" => {
                if let Some(i) = wire.value.as_i64() {
                    Ok(Self::Integer(i))
                } else if let Some(s) = wire.value.as_str() {
                    s.parse::<i64>()
                        .map(Self::Integer)
                        .map_err(|_| D::Error::custom(format!("bad integer {s:?}")))
                } else {
                    Err(D::Error::custom(format!(
                        "bad integer value {:?}",
                        wire.value
                    )))
                }
            }
            "bigint" => {
                let s = wire.value.as_str().ok_or_else(|| {
                    D::Error::custom(format!(
                        "bigint value must be a string, got {:?}",
                        wire.value
                    ))
                })?;
                BigInt::parse_bytes(s.as_bytes(), 10)
                    .map(Self::from_bigint)
                    .ok_or_else(|| D::Error::custom(format!("bad bigint {s:?}")))
            }
            "rational" => {
                let s = wire.value.as_str().ok_or_else(|| {
                    D::Error::custom(format!(
                        "rational value must be a string, got {:?}",
                        wire.value
                    ))
                })?;
                // Display form is `num/den` (or plain integer for whole values).
                if let Some((num_s, den_s)) = s.split_once('/') {
                    let num = BigInt::parse_bytes(num_s.trim().as_bytes(), 10)
                        .ok_or_else(|| D::Error::custom(format!("bad rational numerator {s:?}")))?;
                    let den =
                        BigInt::parse_bytes(den_s.trim().as_bytes(), 10).ok_or_else(|| {
                            D::Error::custom(format!("bad rational denominator {s:?}"))
                        })?;
                    Self::rational(num, den)
                        .ok_or_else(|| D::Error::custom(format!("zero denominator in {s:?}")))
                } else {
                    let i = BigInt::parse_bytes(s.trim().as_bytes(), 10)
                        .ok_or_else(|| D::Error::custom(format!("bad rational {s:?}")))?;
                    Ok(Self::from_bigint(i))
                }
            }
            "decimal" => {
                let s = wire.value.as_str().ok_or_else(|| {
                    D::Error::custom(format!(
                        "decimal value must be a string, got {:?}",
                        wire.value
                    ))
                })?;
                s.parse::<Decimal>()
                    .map(Self::Decimal)
                    .map_err(|_| D::Error::custom(format!("bad decimal {s:?}")))
            }
            "float" => json_to_float(&wire.value)
                .map(Self::Float)
                .ok_or_else(|| D::Error::custom(format!("bad float {:?}", wire.value))),
            "complex" => {
                let re_v = wire.value.get("re").ok_or_else(|| {
                    D::Error::custom(format!("complex needs re/im, got {:?}", wire.value))
                })?;
                let im_v = wire.value.get("im").ok_or_else(|| {
                    D::Error::custom(format!("complex needs re/im, got {:?}", wire.value))
                })?;
                let (re, im) = (json_to_float(re_v), json_to_float(im_v));
                match (re, im) {
                    (Some(r), Some(i)) => Ok(Self::from(Complex64::new(r, i))),
                    _ => Err(D::Error::custom(format!("bad complex {:?}", wire.value))),
                }
            }
            other => Err(D::Error::custom(format!("unknown number kind {other:?}"))),
        }
    }
}

// ----------------------------------------------------------------------
// Unit tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_integers() {
        assert_eq!(Number::from_decimal_str("42").unwrap(), Number::Integer(42));
        assert_eq!(Number::from_decimal_str("-7").unwrap(), Number::Integer(-7));
        assert_eq!(
            Number::from_decimal_str("1_000_000").unwrap(),
            Number::Integer(1_000_000)
        );
    }

    #[test]
    fn parses_radix_integers() {
        assert_eq!(
            Number::from_int_str("0xFF", 16).unwrap(),
            Number::Integer(255)
        );
        assert_eq!(
            Number::from_int_str("0b1010", 2).unwrap(),
            Number::Integer(10)
        );
        assert_eq!(
            Number::from_int_str("0o17", 8).unwrap(),
            Number::Integer(15)
        );
        assert_eq!(
            Number::from_int_str("FF", 16).unwrap(),
            Number::Integer(255)
        );
    }

    #[test]
    fn parses_floats_and_decimals() {
        let n = Number::from_decimal_str("3.14").unwrap();
        assert!(matches!(n, Number::Decimal(_)));
        let n2 = Number::from_decimal_str("1e3").unwrap();
        assert!(!n2.is_zero());
    }

    #[test]
    fn detects_zero_and_integer_valued() {
        assert!(Number::Integer(0).is_zero());
        assert!(Number::Float(0.0).is_zero());
        assert!(Number::Float(3.0).is_integer_valued());
        assert!(!Number::Float(3.5).is_integer_valued());
    }

    #[test]
    fn bigint_promotion_on_overflow_literal() {
        let huge = "99999999999999999999999999";
        let n = Number::from_decimal_str(huge).unwrap();
        assert!(matches!(n, Number::BigInt(_)));
    }

    #[test]
    fn display_roundtrips() {
        assert_eq!(Number::Integer(-5).to_string(), "-5");
        assert_eq!(Number::from_f64(2.0).to_string(), "2.0");
    }

    #[test]
    fn exact_third_stays_rational() {
        let third = Number::Integer(1).div(&Number::Integer(3)).unwrap();
        assert!(matches!(third, Number::Rational(_)));
        assert_eq!(third.to_string(), "1/3");
        let sum = third
            .add(&Number::Integer(1).div(&Number::Integer(3)).unwrap())
            .unwrap()
            .add(&Number::Integer(1).div(&Number::Integer(3)).unwrap())
            .unwrap();
        assert_eq!(sum, Number::Integer(1));
    }

    #[test]
    fn bigint_is_arbitrary_precision() {
        // 100! has 158 digits — far beyond i64/Decimal.
        let mut acc = Number::Integer(1);
        for i in 2..=100i64 {
            acc = acc.mul(&Number::Integer(i)).unwrap();
        }
        let s = acc.to_string();
        assert_eq!(s.len(), 158);
        assert!(s.starts_with("933262154439"));
        assert!(matches!(acc, Number::BigInt(_)));
    }

    #[test]
    fn decimal_is_exact_base10() {
        // 0.1 + 0.2 == 0.3 in Decimal, but not in binary Float.
        let a = Number::from_decimal_str("0.1").unwrap();
        let b = Number::from_decimal_str("0.2").unwrap();
        let s = a.add(&b).unwrap();
        assert_eq!(s.to_string(), "0.3");
        let f = Number::Float(0.1).add(&Number::Float(0.2)).unwrap();
        assert_ne!(f.to_f64(), 0.3);
        assert!((f.to_f64() - 0.3).abs() < 1e-16);
    }

    #[test]
    fn conversion_matrix_roundtrips() {
        let i = Number::Integer(-42);
        assert_eq!(i.to_f64(), -42.0);
        assert_eq!(i.to_decimal().unwrap().to_string(), "-42");
        assert_eq!(i.to_rational().unwrap().to_string(), "-42");
        assert_eq!(i.to_complex(), Complex64::new(-42.0, 0.0));
        assert_eq!(i.as_i64(), Some(-42));
        assert_eq!(i.kind(), "integer");

        let r = Number::from_rational(BigRational::new(BigInt::from(3), BigInt::from(2)));
        assert!(matches!(r, Number::Rational(_)));
        assert_eq!(r.to_f64(), 1.5);
        assert_eq!(r.kind(), "rational");
        assert_eq!(r.as_i64(), None); // not integer-valued

        let c = Number::complex(1.0, -2.0);
        assert!(c.is_complex());
        assert_eq!(c.kind(), "complex");
        assert_eq!(c.to_string(), "1 - 2i");
        assert_eq!(Number::complex(5.0, 0.0), Number::Float(5.0)); // normalised
        assert!(!Number::Float(1.0).is_complex());

        // Rational normalises integers: 4/2 == 2.
        let two = Number::rational(BigInt::from(4), BigInt::from(2)).unwrap();
        assert_eq!(two, Number::Integer(2));
        assert!(Number::rational(BigInt::from(1), BigInt::zero()).is_none());

        // From impls.
        assert_eq!(Number::from(7u64), Number::Integer(7));
        assert!(matches!(Number::from(u64::MAX), Number::BigInt(_)));
        assert_eq!(Number::from(5i128), Number::Integer(5));
    }

    #[test]
    fn complex_display_cases() {
        assert_eq!(Number::complex(0.0, 1.0).to_string(), "1i");
        assert_eq!(Number::complex(0.0, -1.5).to_string(), "-1.5i");
        assert_eq!(Number::complex(2.0, 3.0).to_string(), "2 + 3i");
        assert_eq!(Number::complex(2.0, -3.0).to_string(), "2 - 3i");
    }
}
