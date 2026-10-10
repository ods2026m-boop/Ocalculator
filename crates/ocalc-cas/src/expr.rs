//! Symbolic expression tree.
//!
//! [`SymExpr`] is the CAS counterpart to the engine's numeric AST: it keeps
//! variables unevaluated so [`simplify`](crate::simplify),
//! [`differentiate`](crate::differentiate) and friends can rewrite structure.
//! Build trees with the constructors ([`var`], [`konst`], [`add`], [`mul`],
//! [`pow`], [`sin`], [`cos`], [`log`]); text parsing stays in the engine.

use crate::Symbol;

/// Supported single-argument functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FuncKind {
    /// `sin(x)`.
    Sin,
    /// `cos(x)`.
    Cos,
    /// Natural logarithm `log(x)`.
    Log,
}

impl FuncKind {
    /// Function name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Log => "log",
        }
    }
}

/// A symbolic expression.
#[derive(Debug, Clone, PartialEq)]
pub enum SymExpr {
    /// Numeric constant.
    Const(f64),
    /// Variable.
    Var(String),
    /// Negation `-x`.
    Neg(Box<SymExpr>),
    /// N-ary sum.
    Add(Vec<SymExpr>),
    /// N-ary product.
    Mul(Vec<SymExpr>),
    /// Power `base^exp`.
    Pow(Box<SymExpr>, Box<SymExpr>),
    /// Single-argument function call.
    Func(FuncKind, Box<SymExpr>),
}

/// Variable expression.
#[must_use]
pub fn var(name: &str) -> SymExpr {
    SymExpr::Var(name.to_owned())
}

/// Variable from a [`Symbol`].
#[must_use]
pub fn var_of(sym: &Symbol) -> SymExpr {
    SymExpr::Var(sym.name().to_owned())
}

/// Numeric constant.
#[must_use]
pub fn konst(v: f64) -> SymExpr {
    SymExpr::Const(v)
}

/// Sum of terms.
#[must_use]
pub fn add(terms: Vec<SymExpr>) -> SymExpr {
    SymExpr::Add(terms)
}

/// Product of factors.
#[must_use]
pub fn mul(factors: Vec<SymExpr>) -> SymExpr {
    SymExpr::Mul(factors)
}

/// Power `base^exp`.
#[must_use]
pub fn pow(base: SymExpr, exp: SymExpr) -> SymExpr {
    SymExpr::Pow(Box::new(base), Box::new(exp))
}

/// `sin(x)`.
#[must_use]
pub fn sin(arg: SymExpr) -> SymExpr {
    SymExpr::Func(FuncKind::Sin, Box::new(arg))
}

/// `cos(x)`.
#[must_use]
pub fn cos(arg: SymExpr) -> SymExpr {
    SymExpr::Func(FuncKind::Cos, Box::new(arg))
}

/// Natural logarithm `log(x)`.
#[must_use]
pub fn log(arg: SymExpr) -> SymExpr {
    SymExpr::Func(FuncKind::Log, Box::new(arg))
}

/// Format a constant: integers print without a decimal point.
pub(crate) fn fmt_num(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "inf".to_owned()
        } else {
            "-inf".to_owned()
        };
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    format!("{n}")
}

impl SymExpr {
    /// Precedence level for parenthesization (higher binds tighter).
    fn prec(&self) -> u8 {
        match self {
            Self::Const(_) | Self::Var(_) | Self::Func(..) => 4,
            Self::Pow(..) => 3,
            Self::Neg(_) => 2,
            Self::Mul(_) => 1,
            Self::Add(_) => 0,
        }
    }

    /// Render with parentheses when the child binds looser than `parent`.
    fn child(&self, parent: u8) -> String {
        let s = self.to_string();
        if self.prec() < parent {
            format!("({s})")
        } else {
            s
        }
    }
}

impl std::fmt::Display for SymExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const(n) => write!(f, "{}", fmt_num(*n)),
            Self::Var(name) => write!(f, "{name}"),
            Self::Neg(x) => write!(f, "-{}", x.child(2)),
            Self::Add(terms) => {
                let mut out = String::new();
                for (i, t) in terms.iter().enumerate() {
                    let s = t.to_string();
                    if i == 0 {
                        out.push_str(&s);
                    } else if s.starts_with('-') {
                        out.push_str(" - ");
                        out.push_str(s.trim_start_matches('-'));
                    } else {
                        out.push_str(" + ");
                        out.push_str(&s);
                    }
                }
                write!(f, "{out}")
            }
            Self::Mul(factors) => {
                let parts: Vec<String> = factors.iter().map(|x| x.child(1)).collect();
                write!(f, "{}", parts.join("*"))
            }
            Self::Pow(b, e) => write!(f, "{}^{}", b.child(3), e.child(3)),
            Self::Func(k, a) => write!(f, "{}({a})", k.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_forms() {
        assert_eq!(konst(2.0).to_string(), "2");
        assert_eq!(var("x").to_string(), "x");
        assert_eq!(add(vec![var("x"), konst(1.0)]).to_string(), "x + 1");
        assert_eq!(
            add(vec![var("x"), SymExpr::Neg(Box::new(var("y")))]).to_string(),
            "x - y"
        );
        assert_eq!(pow(var("x"), konst(2.0)).to_string(), "x^2");
        assert_eq!(sin(var("x")).to_string(), "sin(x)");
        // Precedence: (x+1)*y parenthesizes the sum.
        let e = mul(vec![add(vec![var("x"), konst(1.0)]), var("y")]);
        assert_eq!(e.to_string(), "(x + 1)*y");
    }
}
