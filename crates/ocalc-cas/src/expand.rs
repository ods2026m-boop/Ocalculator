//! Expansion: distribute products over sums (`(x+1)^2 → x^2+2*x+1`).

use crate::{simplify, SymExpr};

/// Fully expand products of sums and small integer powers of sums.
#[must_use]
pub fn expand(e: &SymExpr) -> SymExpr {
    match e {
        SymExpr::Mul(factors) => {
            let parts: Vec<SymExpr> = factors.iter().map(expand).collect();
            if parts.iter().any(|f| matches!(f, SymExpr::Add(_))) {
                // Distribute one sum level, then recurse: each round removes
                // one Add factor per path, so this terminates.
                expand(&distribute_mul(&parts))
            } else {
                simplify(&SymExpr::Mul(parts))
            }
        }
        SymExpr::Pow(b, x) => {
            let base = expand(b);
            match x.as_ref() {
                SymExpr::Const(n)
                    if *n >= 0.0
                        && n.fract() == 0.0
                        && *n <= 8.0
                        && matches!(base, SymExpr::Add(_)) =>
                {
                    let mut acc = SymExpr::Const(1.0);
                    for _ in 0..(*n as u64) {
                        acc = distribute_mul(&[acc, base.clone()]);
                    }
                    simplify(&expand(&acc))
                }
                _ => simplify(&SymExpr::Pow(Box::new(base), Box::new((**x).clone()))),
            }
        }
        SymExpr::Add(terms) => simplify(&SymExpr::Add(terms.iter().map(expand).collect())),
        SymExpr::Neg(x) => simplify(&SymExpr::Neg(Box::new(expand(x)))),
        SymExpr::Func(k, a) => simplify(&SymExpr::Func(*k, Box::new(expand(a)))),
        _ => simplify(e),
    }
}

/// Multiply factors, distributing the first `Add` found.
fn distribute_mul(factors: &[SymExpr]) -> SymExpr {
    if factors.is_empty() {
        return SymExpr::Const(1.0);
    }
    // Find the first sum factor.
    let pos = factors.iter().position(|f| matches!(f, SymExpr::Add(_)));
    match pos {
        None => SymExpr::Mul(factors.to_vec()),
        Some(i) => {
            let rest: Vec<SymExpr> = factors
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, f)| f.clone())
                .collect();
            let rest = if rest.is_empty() {
                SymExpr::Const(1.0)
            } else {
                SymExpr::Mul(rest)
            };
            match &factors[i] {
                SymExpr::Add(terms) => SymExpr::Add(
                    terms
                        .iter()
                        .map(|t| distribute_mul(&[t.clone(), rest.clone()]))
                        .collect(),
                ),
                _ => SymExpr::Mul(factors.to_vec()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, konst, mul, pow, var};

    #[test]
    fn square_of_sum() {
        // (x+1)^2 → x^2 + 2*x + 1.
        let e = pow(add(vec![var("x"), konst(1.0)]), konst(2.0));
        assert_eq!(expand(&e).to_string(), "x^2 + 2*x + 1");
    }

    #[test]
    fn product_of_sums() {
        // (x+1)*(x+2) → x^2 + 3*x + 2.
        let e = mul(vec![
            add(vec![var("x"), konst(1.0)]),
            add(vec![var("x"), konst(2.0)]),
        ]);
        assert_eq!(expand(&e).to_string(), "x^2 + 3*x + 2");
    }

    #[test]
    fn leaves_expanded_alone() {
        let e = add(vec![
            pow(var("x"), konst(2.0)),
            mul(vec![konst(2.0), var("x")]),
        ]);
        assert_eq!(expand(&e).to_string(), "x^2 + 2*x");
    }
}
