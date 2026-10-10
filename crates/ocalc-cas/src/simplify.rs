//! Simplification engine: constant folding, identities, like-term
//! collection (`x + x → 2*x`), same-base products (`x*x → x^2`).

use std::collections::BTreeMap;

use crate::{FuncKind, SymExpr};

/// Tolerance below which a coefficient counts as zero.
const EPS: f64 = 1e-12;

/// Simplify `e` (idempotent on simplified input).
#[must_use]
pub fn simplify(e: &SymExpr) -> SymExpr {
    match e {
        SymExpr::Const(_) | SymExpr::Var(_) => e.clone(),
        SymExpr::Neg(x) => {
            let s = simplify(x);
            match s {
                SymExpr::Const(c) => SymExpr::Const(-c),
                SymExpr::Neg(y) => *y,
                other => SymExpr::Neg(Box::new(other)),
            }
        }
        SymExpr::Add(terms) => simplify_add(terms),
        SymExpr::Mul(factors) => simplify_mul(factors),
        SymExpr::Pow(b, x) => simplify_pow(&simplify(b), &simplify(x)),
        SymExpr::Func(k, a) => {
            let s = simplify(a);
            if let SymExpr::Const(c) = s {
                if let Some(v) = eval_func(*k, c) {
                    return SymExpr::Const(v);
                }
            }
            SymExpr::Func(*k, Box::new(s))
        }
    }
}

fn eval_func(k: FuncKind, c: f64) -> Option<f64> {
    match k {
        FuncKind::Sin => Some(c.sin()),
        FuncKind::Cos => Some(c.cos()),
        FuncKind::Log => (c > 0.0).then(|| c.ln()),
    }
}

fn is_zero_const(e: &SymExpr) -> bool {
    matches!(e, SymExpr::Const(c) if c.abs() < EPS)
}

/// Split a simplified term into `(coefficient, base)`; `base == None` means a
/// bare constant. The simplifier keeps the constant factor first in [`Mul`];
/// negation flips the coefficient (`-x` groups with `x`).
fn coeff_split(term: &SymExpr) -> (f64, Option<SymExpr>) {
    match term {
        SymExpr::Const(c) => (*c, None),
        SymExpr::Neg(x) => {
            let (c, base) = coeff_split(x);
            (-c, base)
        }
        SymExpr::Mul(factors) => match factors.first() {
            Some(SymExpr::Const(c)) => {
                let rest = &factors[1..];
                let base = match rest.len() {
                    0 => None,
                    1 => Some(rest[0].clone()),
                    _ => Some(SymExpr::Mul(rest.to_vec())),
                };
                (*c, base)
            }
            _ => (1.0, Some(term.clone())),
        },
        _ => (1.0, Some(term.clone())),
    }
}

fn simplify_add(terms: &[SymExpr]) -> SymExpr {
    // Flatten nested sums, drop zeros.
    let mut flat: Vec<SymExpr> = Vec::new();
    for t in terms {
        match simplify(t) {
            SymExpr::Add(inner) => flat.extend(inner),
            s if is_zero_const(&s) => {}
            s => flat.push(s),
        }
    }
    let mut const_sum = 0.0;
    let mut groups: BTreeMap<String, (f64, SymExpr)> = BTreeMap::new();
    for t in &flat {
        let (c, base) = coeff_split(t);
        match base {
            None => const_sum += c,
            Some(b) => {
                let key = b.to_string();
                groups
                    .entry(key)
                    .and_modify(|slot| slot.0 += c)
                    .or_insert((c, b));
            }
        }
    }
    // Canonical order: longer (higher-degree) bases first, then lexical.
    // Constants always trail, so `x^2 + 2*x + 1` reads naturally.
    let mut grouped: Vec<(String, (f64, SymExpr))> = groups.into_iter().collect();
    grouped.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    let mut out: Vec<SymExpr> = Vec::new();
    for (_, (c, base)) in grouped {
        if c.abs() < EPS {
            continue;
        }
        out.push(mul_const(c, base));
    }
    if const_sum.abs() >= EPS {
        out.push(SymExpr::Const(const_sum));
    }
    match out.len() {
        0 => SymExpr::Const(0.0),
        1 => out.pop().unwrap_or(SymExpr::Const(0.0)),
        _ => SymExpr::Add(out),
    }
}

/// `c * base` with `c == 1` elided and `c == -1` as negation.
fn mul_const(c: f64, base: SymExpr) -> SymExpr {
    if (c - 1.0).abs() < EPS {
        base
    } else if (c + 1.0).abs() < EPS {
        SymExpr::Neg(Box::new(base))
    } else {
        SymExpr::Mul(vec![SymExpr::Const(c), base])
    }
}

/// Split a simplified factor into `(base, exponent)` for same-base grouping.
fn pow_split(f: &SymExpr) -> (String, SymExpr, f64) {
    match f {
        SymExpr::Pow(b, e) => match e.as_ref() {
            SymExpr::Const(n) => (b.to_string(), (**b).clone(), *n),
            _ => (f.to_string(), f.clone(), 1.0),
        },
        _ => (f.to_string(), f.clone(), 1.0),
    }
}

fn simplify_mul(factors: &[SymExpr]) -> SymExpr {
    let mut flat: Vec<SymExpr> = Vec::new();
    for x in factors {
        match simplify(x) {
            SymExpr::Mul(inner) => flat.extend(inner),
            s => flat.push(s),
        }
    }
    let mut product = 1.0;
    let mut rest: Vec<SymExpr> = Vec::new();
    for x in &flat {
        match x {
            SymExpr::Const(c) => product *= *c,
            _ => rest.push(x.clone()),
        }
    }
    if product.abs() < EPS {
        return SymExpr::Const(0.0);
    }
    // Same-base exponents: x*x → x^2, x^2*x^3 → x^5.
    let mut groups: BTreeMap<String, (SymExpr, f64)> = BTreeMap::new();
    for x in &rest {
        let (key, base, exp) = pow_split(x);
        groups
            .entry(key)
            .and_modify(|slot| slot.1 += exp)
            .or_insert((base, exp));
    }
    let mut out: Vec<SymExpr> = Vec::new();
    if (product - 1.0).abs() >= EPS || groups.is_empty() {
        out.push(SymExpr::Const(product));
    }
    for (_, (base, exp)) in groups {
        if exp.abs() < EPS {
            continue;
        }
        if (exp - 1.0).abs() < EPS {
            out.push(base);
        } else {
            out.push(SymExpr::Pow(Box::new(base), Box::new(SymExpr::Const(exp))));
        }
    }
    match out.len() {
        0 => SymExpr::Const(1.0),
        1 => out.pop().unwrap_or(SymExpr::Const(1.0)),
        _ => SymExpr::Mul(out),
    }
}

fn simplify_pow(b: &SymExpr, x: &SymExpr) -> SymExpr {
    if let SymExpr::Const(n) = x {
        if n.abs() < EPS {
            return SymExpr::Const(1.0);
        }
        if (*n - 1.0).abs() < EPS {
            return b.clone();
        }
    }
    // (x^m)^n → x^(m·n).
    if let (SymExpr::Pow(inner, exp), SymExpr::Const(n)) = (b, x) {
        if let SymExpr::Const(m) = exp.as_ref() {
            return SymExpr::Pow(Box::new((**inner).clone()), Box::new(SymExpr::Const(m * n)));
        }
    }
    match (b, x) {
        (SymExpr::Const(a), SymExpr::Const(n)) => {
            // Keep exact integer powers exact; guard the rest.
            if *n >= 0.0 && n.fract() == 0.0 && *n <= 64.0 && a.fract() == 0.0 {
                SymExpr::Const(a.powf(*n))
            } else if *a == 0.0 {
                SymExpr::Const(0.0)
            } else if *a == 1.0 {
                SymExpr::Const(1.0)
            } else {
                SymExpr::Pow(Box::new(b.clone()), Box::new(x.clone()))
            }
        }
        (SymExpr::Const(a), _) if *a == 0.0 => SymExpr::Const(0.0),
        (SymExpr::Const(a), _) if *a == 1.0 => SymExpr::Const(1.0),
        _ => SymExpr::Pow(Box::new(b.clone()), Box::new(x.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, cos, konst, log, mul, pow, sin, var};

    fn v(name: &str) -> SymExpr {
        var(name)
    }

    #[test]
    fn combines_like_terms() {
        // x + x → 2*x.
        let e = simplify(&add(vec![v("x"), v("x")]));
        assert_eq!(e.to_string(), "2*x");
        // 2x + 3x → 5x.
        let e = simplify(&add(vec![
            mul(vec![konst(2.0), v("x")]),
            mul(vec![konst(3.0), v("x")]),
        ]));
        assert_eq!(e.to_string(), "5*x");
        // x - x → 0.
        let e = simplify(&add(vec![v("x"), SymExpr::Neg(Box::new(v("x")))]));
        assert_eq!(e, SymExpr::Const(0.0));
    }

    #[test]
    fn same_base_products() {
        // x*x → x^2.
        let e = simplify(&mul(vec![v("x"), v("x")]));
        assert_eq!(e.to_string(), "x^2");
        // 2*x^2*3*x^3 → 6*x^5.
        let e = simplify(&mul(vec![
            konst(2.0),
            pow(v("x"), konst(2.0)),
            konst(3.0),
            pow(v("x"), konst(3.0)),
        ]));
        assert_eq!(e.to_string(), "6*x^5");
    }

    #[test]
    fn identities() {
        assert_eq!(simplify(&add(vec![v("x"), konst(0.0)])).to_string(), "x");
        assert_eq!(simplify(&mul(vec![v("x"), konst(1.0)])).to_string(), "x");
        assert_eq!(
            simplify(&mul(vec![v("x"), konst(0.0)])),
            SymExpr::Const(0.0)
        );
        assert_eq!(simplify(&pow(v("x"), konst(1.0))).to_string(), "x");
        assert_eq!(simplify(&pow(v("x"), konst(0.0))), SymExpr::Const(1.0));
        // --x → x.
        let e = simplify(&SymExpr::Neg(Box::new(SymExpr::Neg(Box::new(v("x"))))));
        assert_eq!(e.to_string(), "x");
    }

    #[test]
    fn folds_constants() {
        assert_eq!(
            simplify(&add(vec![konst(2.0), konst(3.0)])),
            SymExpr::Const(5.0)
        );
        assert_eq!(
            simplify(&pow(konst(2.0), konst(10.0))),
            SymExpr::Const(1024.0)
        );
        assert!((simplify(&sin(konst(0.0))) == SymExpr::Const(0.0)));
        assert!(simplify(&log(konst(-1.0))).to_string().starts_with("log("));
        assert_eq!(simplify(&cos(konst(0.0))), SymExpr::Const(1.0));
    }

    #[test]
    fn idempotent() {
        let e = add(vec![
            mul(vec![konst(2.0), pow(v("x"), konst(2.0))]),
            mul(vec![konst(3.0), v("x")]),
            konst(1.0),
        ]);
        let once = simplify(&e);
        assert_eq!(simplify(&once), once);
    }
}
