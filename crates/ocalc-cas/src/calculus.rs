//! Differentiation and integration foundation.
//!
//! [`differentiate`] implements sum, product, power and chain rules for the
//! [`SymExpr`] tree (`d/dx x^2 = 2*x`). [`integrate`] covers linearity plus
//! the power rule and direct `sin`/`cos`/`log` forms (`integral x dx =
//! x^2/2`); anything else is an honest `Err`, not a guess.

use crate::{simplify, FuncKind, SymExpr};

/// Differentiate `e` with respect to `var`.
///
/// # Errors
/// Variable exponents (`d/dx x^y`) and unknown call shapes.
pub fn differentiate(e: &SymExpr, var: &str) -> Result<SymExpr, String> {
    let d = diff(e, var)?;
    Ok(simplify(&d))
}

fn diff(e: &SymExpr, var: &str) -> Result<SymExpr, String> {
    match e {
        SymExpr::Const(_) => Ok(SymExpr::Const(0.0)),
        SymExpr::Var(name) => Ok(SymExpr::Const(f64::from(name == var))),
        SymExpr::Neg(x) => Ok(SymExpr::Neg(Box::new(diff(x, var)?))),
        SymExpr::Add(ts) => {
            let mut out = Vec::with_capacity(ts.len());
            for t in ts {
                out.push(diff(t, var)?);
            }
            Ok(SymExpr::Add(out))
        }
        SymExpr::Mul(fs) => {
            // Product rule over N factors: Σ fᵢ′·Πⱼ≠ᵢ fⱼ.
            let mut sum = Vec::with_capacity(fs.len());
            for (i, f) in fs.iter().enumerate() {
                let mut term: Vec<SymExpr> = vec![diff(f, var)?];
                for (j, g) in fs.iter().enumerate() {
                    if i != j {
                        term.push(g.clone());
                    }
                }
                sum.push(SymExpr::Mul(term));
            }
            Ok(SymExpr::Add(sum))
        }
        SymExpr::Pow(b, x) => match x.as_ref() {
            SymExpr::Const(n) => {
                // d/dx f^n = n·f^(n-1)·f′.
                Ok(SymExpr::Mul(vec![
                    SymExpr::Const(*n),
                    SymExpr::Pow(Box::new((**b).clone()), Box::new(SymExpr::Const(n - 1.0))),
                    diff(b, var)?,
                ]))
            }
            _ => match b.as_ref() {
                SymExpr::Const(_) => {
                    // d/dx c^g = c^g·ln(c)·g′.
                    Ok(SymExpr::Mul(vec![
                        e.clone(),
                        SymExpr::Func(FuncKind::Log, Box::new((**b).clone())),
                        diff(x, var)?,
                    ]))
                }
                _ => Err(format!(
                    "cannot differentiate {e} with respect to {var}: variable exponent"
                )),
            },
        },
        SymExpr::Func(k, a) => {
            let inner = diff(a, var)?;
            let outer = match k {
                // d/dx sin(u) = cos(u)·u′.
                FuncKind::Sin => SymExpr::Func(FuncKind::Cos, a.clone()),
                // d/dx cos(u) = -sin(u)·u′.
                FuncKind::Cos => SymExpr::Neg(Box::new(SymExpr::Func(FuncKind::Sin, a.clone()))),
                // d/dx log(u) = u′/u.
                FuncKind::Log => SymExpr::Pow(a.clone(), Box::new(SymExpr::Const(-1.0))),
            };
            Ok(SymExpr::Mul(vec![outer, inner]))
        }
    }
}

/// Indefinite integral of `e` with respect to `var` (no `+ C` constant — the
/// caller adds it when displaying).
///
/// Covers constants, the variable, linearity, `x^n` (power rule, `n ≠ -1`),
/// `1/x`, and direct `sin(x)`/`cos(x)`/`log(x)` forms.
///
/// # Errors
/// Anything outside that foundation (products of non-constants, nested
/// calls, variable exponents, …).
pub fn integrate(e: &SymExpr, var: &str) -> Result<SymExpr, String> {
    let i = integ(&simplify(e), var)?;
    Ok(simplify(&i))
}

fn integ(e: &SymExpr, var: &str) -> Result<SymExpr, String> {
    match e {
        SymExpr::Const(c) => Ok(SymExpr::Mul(vec![
            SymExpr::Const(*c),
            SymExpr::Var(var.to_owned()),
        ])),
        SymExpr::Var(name) if name == var => Ok(SymExpr::Mul(vec![
            SymExpr::Const(0.5),
            SymExpr::Pow(Box::new(e.clone()), Box::new(SymExpr::Const(2.0))),
        ])),
        SymExpr::Var(_) => Ok(SymExpr::Mul(vec![e.clone(), SymExpr::Var(var.to_owned())])),
        SymExpr::Neg(x) => Ok(SymExpr::Neg(Box::new(integ(x, var)?))),
        SymExpr::Add(ts) => {
            let mut out = Vec::with_capacity(ts.len());
            for t in ts {
                out.push(integ(t, var)?);
            }
            Ok(SymExpr::Add(out))
        }
        SymExpr::Mul(fs) => {
            // Linearity: pull out the constant part, integrate the rest.
            let (consts, rest): (Vec<_>, Vec<_>) =
                fs.iter().partition(|f| matches!(f, SymExpr::Const(_)));
            if rest.len() == fs.len() {
                return Err(format!("cannot integrate {e} with respect to {var} yet"));
            }
            let c: f64 = consts
                .iter()
                .filter_map(|f| match f {
                    SymExpr::Const(n) => Some(*n),
                    _ => None,
                })
                .product();
            let inner = match rest.len() {
                1 => rest[0].clone(),
                _ => SymExpr::Mul(rest.into_iter().cloned().collect()),
            };
            Ok(SymExpr::Mul(vec![SymExpr::Const(c), integ(&inner, var)?]))
        }
        SymExpr::Pow(b, x) => match (b.as_ref(), x.as_ref()) {
            (SymExpr::Var(name), SymExpr::Const(n)) if name == var => {
                if (*n + 1.0).abs() < 1e-12 {
                    // ∫x⁻¹dx = log(x).
                    Ok(SymExpr::Func(
                        FuncKind::Log,
                        Box::new(SymExpr::Var(var.to_owned())),
                    ))
                } else {
                    Ok(SymExpr::Mul(vec![
                        SymExpr::Const(1.0 / (n + 1.0)),
                        SymExpr::Pow(
                            Box::new(SymExpr::Var(var.to_owned())),
                            Box::new(SymExpr::Const(n + 1.0)),
                        ),
                    ]))
                }
            }
            _ => Err(format!("cannot integrate {e} with respect to {var} yet")),
        },
        SymExpr::Func(k, a) => match (k, a.as_ref()) {
            (FuncKind::Sin, SymExpr::Var(name)) if name == var => Ok(SymExpr::Neg(Box::new(
                SymExpr::Func(FuncKind::Cos, Box::new(SymExpr::Var(var.to_owned()))),
            ))),
            (FuncKind::Cos, SymExpr::Var(name)) if name == var => Ok(SymExpr::Func(
                FuncKind::Sin,
                Box::new(SymExpr::Var(var.to_owned())),
            )),
            (FuncKind::Log, SymExpr::Var(name)) if name == var => {
                // ∫log(x)dx = x·log(x) − x.
                Ok(SymExpr::Add(vec![
                    SymExpr::Mul(vec![
                        SymExpr::Var(var.to_owned()),
                        SymExpr::Func(FuncKind::Log, Box::new(SymExpr::Var(var.to_owned()))),
                    ]),
                    SymExpr::Neg(Box::new(SymExpr::Var(var.to_owned()))),
                ]))
            }
            _ => Err(format!("cannot integrate {e} with respect to {var} yet")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, konst, log, mul, pow, sin, var};

    #[test]
    fn power_rule() {
        // d/dx x^2 = 2*x.
        let d = differentiate(&pow(var("x"), konst(2.0)), "x").unwrap();
        assert_eq!(d.to_string(), "2*x");
        // d/dx x = 1, d/dx 5 = 0, d/dy x = 0.
        assert_eq!(differentiate(&var("x"), "x").unwrap(), SymExpr::Const(1.0));
        assert_eq!(
            differentiate(&konst(5.0), "x").unwrap(),
            SymExpr::Const(0.0)
        );
        assert_eq!(differentiate(&var("x"), "y").unwrap(), SymExpr::Const(0.0));
    }

    #[test]
    fn sum_product_chain() {
        // d/dx (x^2 + 3x) = 2*x + 3.
        let e = add(vec![
            pow(var("x"), konst(2.0)),
            mul(vec![konst(3.0), var("x")]),
        ]);
        assert_eq!(differentiate(&e, "x").unwrap().to_string(), "2*x + 3");
        // d/dx sin(x) = cos(x); d/dx log(x) keeps 1/x form.
        assert_eq!(
            differentiate(&sin(var("x")), "x").unwrap().to_string(),
            "cos(x)"
        );
        assert_eq!(
            differentiate(&log(var("x")), "x").unwrap().to_string(),
            "x^-1"
        );
    }

    #[test]
    fn rejects_variable_exponent() {
        assert!(differentiate(&pow(var("x"), var("y")), "x").is_err());
    }

    #[test]
    fn power_rule_integral() {
        // integral x dx = 0.5*x^2.
        let i = integrate(&var("x"), "x").unwrap();
        assert_eq!(i.to_string(), "0.5*x^2");
        // integral (2x + 1) dx = x^2 + x.
        let e = add(vec![mul(vec![konst(2.0), var("x")]), konst(1.0)]);
        assert_eq!(integrate(&e, "x").unwrap().to_string(), "x^2 + x");
        // integral sin(x) dx = -cos(x).
        assert_eq!(
            integrate(&sin(var("x")), "x").unwrap().to_string(),
            "-cos(x)"
        );
    }

    #[test]
    fn rejects_products_and_nesting() {
        assert!(integrate(&mul(vec![var("x"), sin(var("x"))]), "x").is_err());
        assert!(integrate(&sin(pow(var("x"), konst(2.0))), "x").is_err());
    }
}
