//! Substitution: replace a variable with an expression, then simplify.

use crate::{simplify, SymExpr};

/// Replace every free occurrence of `var` in `e` with `value`, simplified.
#[must_use]
pub fn substitute(e: &SymExpr, var: &str, value: &SymExpr) -> SymExpr {
    simplify(&replace(e, var, value))
}

fn replace(e: &SymExpr, var: &str, value: &SymExpr) -> SymExpr {
    match e {
        SymExpr::Const(_) => e.clone(),
        SymExpr::Var(name) if name == var => value.clone(),
        SymExpr::Var(_) => e.clone(),
        SymExpr::Neg(x) => SymExpr::Neg(Box::new(replace(x, var, value))),
        SymExpr::Add(ts) => SymExpr::Add(ts.iter().map(|t| replace(t, var, value)).collect()),
        SymExpr::Mul(fs) => SymExpr::Mul(fs.iter().map(|x| replace(x, var, value)).collect()),
        SymExpr::Pow(b, x) => SymExpr::Pow(
            Box::new(replace(b, var, value)),
            Box::new(replace(x, var, value)),
        ),
        SymExpr::Func(k, a) => SymExpr::Func(*k, Box::new(replace(a, var, value))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, konst, mul, pow, var};

    #[test]
    fn replaces_and_simplifies() {
        // (x^2 + 2x)[x := 3] → 15.
        let e = add(vec![
            pow(var("x"), konst(2.0)),
            mul(vec![konst(2.0), var("x")]),
        ]);
        assert_eq!(substitute(&e, "x", &konst(3.0)), SymExpr::Const(15.0));
        // Other variables untouched (terms lead, constants trail).
        let e = add(vec![var("x"), var("y")]);
        assert_eq!(substitute(&e, "x", &konst(1.0)).to_string(), "y + 1");
    }
}
