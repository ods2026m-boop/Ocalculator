//! Factorization foundation: common-factor extraction + integer-root
//! quadratics (`x^2 - 4 → (x-2)*(x+2)`).
//!
//! Candidates are verified by re-expansion: a factored form is returned only
//! when [`expand`](crate::expand)ing it reproduces the simplified input.

use crate::{expand, simplify, SymExpr};

/// Factor `e` (best effort). Returns the simplified input when no
/// factorization applies.
#[must_use]
pub fn factor(e: &SymExpr) -> SymExpr {
    let s = simplify(e);
    if let Some(f) = factor_common(&s) {
        return f;
    }
    if let Some(f) = factor_quadratic(&s) {
        return f;
    }
    s
}

/// Factor out an integer GCD and common variable powers from a sum.
fn factor_common(s: &SymExpr) -> Option<SymExpr> {
    let terms = match s {
        SymExpr::Add(t) => t.clone(),
        _ => return None,
    };
    if terms.len() < 2 {
        return None;
    }
    // Integer GCD of the term coefficients (rounded, verified later).
    let coeffs: Vec<i64> = terms.iter().map(|t| coeff_int(t).unwrap_or(0)).collect();
    let mut g: i64 = 0;
    for c in &coeffs {
        g = gcd(g, c.abs());
    }
    // Common variable powers intersected over ALL terms: a bare constant
    // contributes no powers, so `2*x + 4` never factors out `x`.
    let mut common: Vec<(String, f64)> = var_powers(&terms[0]);
    for t in &terms[1..] {
        let have = var_powers(t);
        let mut next: Vec<(String, f64)> = Vec::new();
        for (name, e) in &common {
            if let Some((_, e2)) = have.iter().find(|(n, _)| n == name) {
                let m = e.min(*e2);
                if m > 0.0 {
                    next.push((name.clone(), m));
                }
            }
        }
        common = next;
    }
    if g <= 1 && common.is_empty() {
        return None;
    }
    // Build factor * rest, verify by re-expansion.
    let mut outer: Vec<SymExpr> = Vec::new();
    if g > 1 {
        outer.push(SymExpr::Const(g as f64));
    }
    for (name, e) in &common {
        if (*e - 1.0).abs() < 1e-12 {
            outer.push(SymExpr::Var(name.clone()));
        } else {
            outer.push(SymExpr::Pow(
                Box::new(SymExpr::Var(name.clone())),
                Box::new(SymExpr::Const(*e)),
            ));
        }
    }
    let rest: Vec<SymExpr> = terms.iter().map(|t| divide_term(t, g, &common)).collect();
    let candidate = simplify(&SymExpr::Mul(vec![
        if outer.len() == 1 {
            outer.pop().unwrap_or(SymExpr::Const(1.0))
        } else {
            SymExpr::Mul(outer)
        },
        SymExpr::Add(rest),
    ]));
    if expand(&candidate) == expand(s) && candidate != *s {
        Some(candidate)
    } else {
        None
    }
}

/// Integer coefficient of a term (leading const, rounded), if integral.
fn coeff_int(t: &SymExpr) -> Option<i64> {
    let c = match t {
        SymExpr::Const(n) => *n,
        SymExpr::Neg(x) => return coeff_int(x).map(|k| -k),
        SymExpr::Mul(fs) => match fs.first() {
            Some(SymExpr::Const(n)) => *n,
            Some(SymExpr::Neg(_)) => -1.0,
            _ => 1.0,
        },
        _ => 1.0,
    };
    if (c - c.round()).abs() < 1e-9 && c.abs() < 1e12 {
        Some(c.round() as i64)
    } else {
        None
    }
}

/// `(var, exponent)` pairs multiplied in a term.
fn var_powers(t: &SymExpr) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    let mut walk = |f: &SymExpr| match f {
        SymExpr::Var(n) => out.push((n.clone(), 1.0)),
        SymExpr::Pow(b, e) => {
            if let (SymExpr::Var(n), SymExpr::Const(k)) = (b.as_ref(), e.as_ref()) {
                out.push((n.clone(), *k));
            }
        }
        _ => {}
    };
    match t {
        SymExpr::Mul(fs) => {
            for f in fs {
                if !matches!(f, SymExpr::Const(_)) {
                    walk(f);
                }
            }
        }
        SymExpr::Neg(x) => {
            for (n, k) in var_powers(x) {
                out.push((n, k));
            }
        }
        SymExpr::Var(n) => out.push((n.clone(), 1.0)),
        SymExpr::Pow(b, e) => {
            if let (SymExpr::Var(n), SymExpr::Const(k)) = (b.as_ref(), e.as_ref()) {
                out.push((n.clone(), *k));
            }
        }
        _ => {}
    }
    out
}

/// Divide a term by the extracted factor (exact for verified candidates).
fn divide_term(t: &SymExpr, g: i64, common: &[(String, f64)]) -> SymExpr {
    let mut e = t.clone();
    if g > 1 {
        e = match e {
            SymExpr::Const(n) => SymExpr::Const(n / g as f64),
            SymExpr::Mul(mut fs) => {
                if let Some(SymExpr::Const(n)) = fs.first() {
                    fs[0] = SymExpr::Const(n / g as f64);
                }
                SymExpr::Mul(fs)
            }
            SymExpr::Neg(x) => SymExpr::Neg(Box::new(divide_term(&x, g, &[]))),
            other => SymExpr::Mul(vec![SymExpr::Const(1.0 / g as f64), other]),
        };
    }
    for (name, k) in common {
        e = strip_power(&e, name, *k);
    }
    simplify(&e)
}

/// Remove `var^k` from a term known to contain it.
fn strip_power(t: &SymExpr, name: &str, k: f64) -> SymExpr {
    match t {
        SymExpr::Var(n) if n == name => {
            if (k - 1.0).abs() < 1e-12 {
                SymExpr::Const(1.0)
            } else {
                SymExpr::Pow(Box::new(t.clone()), Box::new(SymExpr::Const(1.0 - k)))
            }
        }
        SymExpr::Pow(b, e) => {
            if let (SymExpr::Var(n), SymExpr::Const(m)) = (b.as_ref(), e.as_ref()) {
                if n == name {
                    let left = m - k;
                    if left.abs() < 1e-12 {
                        return SymExpr::Const(1.0);
                    }
                    return SymExpr::Pow(Box::new((**b).clone()), Box::new(SymExpr::Const(left)));
                }
            }
            t.clone()
        }
        SymExpr::Mul(fs) => {
            let mut fs = fs.clone();
            for f in &mut fs {
                let next = strip_power(f, name, k);
                // Only strip from the first matching factor.
                if next != *f {
                    *f = next;
                    break;
                }
            }
            fs.retain(|f| !matches!(f, SymExpr::Const(n) if (*n - 1.0).abs() < 1e-12));
            if fs.is_empty() {
                SymExpr::Const(1.0)
            } else if fs.len() == 1 {
                fs.pop().unwrap_or(SymExpr::Const(1.0))
            } else {
                SymExpr::Mul(fs)
            }
        }
        _ => t.clone(),
    }
}

/// Accept a variable name into the single-var slot (empty = anonymous const
/// factor, always fine). Returns false on a second distinct variable.
fn check_var(slot: &mut Option<String>, name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    match slot {
        Some(prev) => prev == name,
        None => {
            *slot = Some(name.to_owned());
            true
        }
    }
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}

/// Factor integer-coefficient quadratics with integer roots:
/// `x^2 - 4 → (x-2)*(x+2)`. Missing degrees default to coefficient 0, so
/// two-term forms (`x^2 - 4`, `x^2 + 2*x`) qualify too.
fn factor_quadratic(s: &SymExpr) -> Option<SymExpr> {
    let terms = match s {
        SymExpr::Add(t) if t.len() == 2 || t.len() == 3 => t.clone(),
        _ => return None,
    };
    // Identify x², x, const parts (single variable).
    let mut var_name: Option<String> = None;
    let (mut a, mut b, mut c) = (0i64, 0i64, 0i64);
    for t in &terms {
        match quad_part(t) {
            Some((name, 2, k)) => {
                if !check_var(&mut var_name, &name) {
                    return None;
                }
                a += k;
            }
            Some((name, 1, k)) => {
                if !check_var(&mut var_name, &name) {
                    return None;
                }
                b += k;
            }
            Some((_, 0, k)) => c += k,
            _ => return None,
        }
    }
    let name = var_name?;
    if a == 0 {
        return None;
    }
    let disc = b * b - 4 * a * c;
    if disc < 0 {
        return None;
    }
    let root = (disc as f64).sqrt();
    if (root - root.round()).abs() > 1e-9 {
        return None;
    }
    let root = root.round() as i64;
    let (num1, num2) = (-b + root, -b - root);
    let den = 2 * a;
    if num1 % den != 0 || num2 % den != 0 {
        return None;
    }
    let (r1, r2) = (num1 / den, num2 / den);
    let x = SymExpr::Var(name);
    // Linear factor (x - r); r == 0 degrades to x.
    let lin = |r: i64| {
        if r == 0 {
            x.clone()
        } else {
            SymExpr::Add(vec![x.clone(), SymExpr::Const(-(r as f64))])
        }
    };
    let candidate = if a == 1 {
        simplify(&SymExpr::Mul(vec![lin(r1), lin(r2)]))
    } else {
        simplify(&SymExpr::Mul(vec![
            SymExpr::Const(a as f64),
            lin(r1),
            lin(r2),
        ]))
    };
    if expand(&candidate) == expand(s) {
        Some(candidate)
    } else {
        None
    }
}

/// Classify a sum term as `(var, degree, integer coefficient)`.
fn quad_part(t: &SymExpr) -> Option<(String, u32, i64)> {
    match t {
        SymExpr::Const(n) if (n - n.round()).abs() < 1e-9 => {
            Some((String::new(), 0, n.round() as i64))
        }
        SymExpr::Var(n) => Some((n.clone(), 1, 1)),
        SymExpr::Neg(x) => {
            let (n, d, k) = quad_part(x)?;
            Some((n, d, -k))
        }
        SymExpr::Mul(fs) => {
            let (mut k, mut name, mut deg) = (1i64, None, 0u32);
            for f in fs {
                match f {
                    SymExpr::Const(n) if (n - n.round()).abs() < 1e-9 => k *= n.round() as i64,
                    SymExpr::Var(n) => {
                        if name.replace(n.clone()).is_some() {
                            return None;
                        }
                        deg += 1;
                    }
                    SymExpr::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
                        (SymExpr::Var(n), SymExpr::Const(m))
                            if m.fract() == 0.0 && *m > 0.0 && *m <= 8.0 =>
                        {
                            if name.replace(n.clone()).is_some() {
                                return None;
                            }
                            deg += *m as u32;
                        }
                        _ => return None,
                    },
                    _ => return None,
                }
            }
            if deg > 2 {
                return None;
            }
            Some((name.unwrap_or_default(), deg, k))
        }
        SymExpr::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
            (SymExpr::Var(n), SymExpr::Const(m)) if *m == 2.0 => Some((n.clone(), 2, 1)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, konst, mul, pow, var};

    #[test]
    fn factors_common_terms() {
        // 2x + 4 → 2*(x + 2).
        let e = add(vec![mul(vec![konst(2.0), var("x")]), konst(4.0)]);
        assert_eq!(factor(&e).to_string(), "2*(x + 2)");
        // x^2 + 2x → x*(x + 2).
        let e = add(vec![
            pow(var("x"), konst(2.0)),
            mul(vec![konst(2.0), var("x")]),
        ]);
        assert_eq!(factor(&e).to_string(), "x*(x + 2)");
    }

    #[test]
    fn factors_difference_of_squares() {
        // x^2 - 4 → (x-2)*(x+2) — display renders (x + -2)*(x + 2).
        let e = add(vec![
            pow(var("x"), konst(2.0)),
            SymExpr::Neg(Box::new(konst(4.0))),
        ]);
        let f = factor(&e);
        assert_eq!(expand(&f), expand(&e));
        assert!(f.to_string().contains("x + 2") || f.to_string().contains("(x"));
    }

    #[test]
    fn leaves_prime_alone() {
        // x^2 + 1 has no real factorization.
        let e = add(vec![pow(var("x"), konst(2.0)), konst(1.0)]);
        assert_eq!(factor(&e), simplify(&e));
        // Coprime linear sum stays.
        let e = add(vec![var("x"), var("y")]);
        assert_eq!(factor(&e).to_string(), "x + y");
    }
}
