//! `cas` plugin module: numeric bridge into the engine.
//!
//! Symbolic entry points ([`simplify`](crate::simplify),
//! [`differentiate`](crate::differentiate), …) are a native Rust API — engine
//! expressions evaluate to [`Number`](ocalc_number::Number) and cannot carry
//! symbolic trees. This provider exposes the numeric kernels instead:
//! `poly_eval`, `solve_linear`, `quadratic_disc`, `quadratic_root`.

use std::sync::Arc;

use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

use crate::{discriminant, solve_linear, solve_quadratic};

/// Provider names (lowercase, sorted).
const FUNCTIONS: [&str; 4] = [
    "poly_eval",
    "quadratic_disc",
    "quadratic_root",
    "solve_linear",
];

/// Numeric CAS kernels as a [`FunctionProvider`].
#[derive(Debug, Clone, Default)]
pub struct CasProvider;

impl CasProvider {
    /// Shared instance (modules hold one each).
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn dispatch(name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        match name {
            "poly_eval" => Some(poly_eval(args)),
            "solve_linear" => Some(do_solve_linear(args)),
            "quadratic_disc" => Some(do_disc(args)),
            "quadratic_root" => Some(do_root(args)),
            _ => None,
        }
    }
}

impl FunctionProvider for CasProvider {
    fn function_names(&self) -> Vec<String> {
        FUNCTIONS.iter().map(|s| (*s).to_owned()).collect()
    }

    fn provides(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        FUNCTIONS.iter().any(|f| *f == key)
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        Self::dispatch(&name.to_ascii_lowercase(), args)
    }
}

/// `poly_eval(x, c0, c1, …)` — Horner evaluation of `c0 + c1·x + …`.
fn poly_eval(args: &[Number]) -> Result<Number, String> {
    if args.len() < 2 {
        return Err("poly_eval(x, c0, c1, …) needs at least x and one coefficient".to_owned());
    }
    let x = args[0].to_f64();
    let mut acc = 0.0;
    for c in args[1..].iter().rev() {
        acc = acc * x + c.to_f64();
    }
    Ok(Number::from_f64(acc))
}

/// `solve_linear(a, b)` — solution of `a·x + b = 0`.
fn do_solve_linear(args: &[Number]) -> Result<Number, String> {
    if args.len() != 2 {
        return Err("solve_linear(a, b) solves a·x + b = 0".to_owned());
    }
    solve_linear(args[0].to_f64(), args[1].to_f64()).map(Number::from_f64)
}

/// `quadratic_disc(a, b, c)` — discriminant `b² − 4ac`.
fn do_disc(args: &[Number]) -> Result<Number, String> {
    if args.len() != 3 {
        return Err("quadratic_disc(a, b, c) needs three coefficients".to_owned());
    }
    Ok(Number::from_f64(discriminant(
        args[0].to_f64(),
        args[1].to_f64(),
        args[2].to_f64(),
    )))
}

/// `quadratic_root(a, b, c, k)` — k-th real root (`k = 1 | 2`, ascending).
fn do_root(args: &[Number]) -> Result<Number, String> {
    if args.len() != 4 {
        return Err("quadratic_root(a, b, c, k) needs coefficients plus k = 1 | 2".to_owned());
    }
    let roots = solve_quadratic(args[0].to_f64(), args[1].to_f64(), args[2].to_f64())?;
    let k = args[3].to_f64();
    if k == 1.0 {
        roots
            .first()
            .copied()
            .map(Number::from_f64)
            .ok_or_else(|| "quadratic has no real roots (discriminant < 0)".to_owned())
    } else if k == 2.0 {
        roots
            .get(1)
            .copied()
            .map(Number::from_f64)
            .ok_or_else(|| "quadratic has no second real root".to_owned())
    } else {
        Err("k must be 1 or 2".to_owned())
    }
}

/// `cas` calculator module.
#[derive(Debug, Clone)]
pub struct CasModule {
    provider: Arc<CasProvider>,
}

impl Default for CasModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CasModule {
    /// New module (runs a symbolic self-test on [`init`](CalculatorModule::init)).
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: Arc::new(CasProvider::new()),
        }
    }
}

impl CalculatorModule for CasModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "cas",
            env!("CARGO_PKG_VERSION"),
            "computer algebra: simplify, calculus, solving",
        )
        .with_capability(Capability::Custom("cas".into()))
    }

    fn init(&self) -> Result<(), String> {
        // Symbolic self-test: d/dx x^2 must be 2*x.
        let d = crate::differentiate(&crate::pow(crate::var("x"), crate::konst(2.0)), "x")
            .map_err(|e| format!("CAS self-test failed: {e}"))?;
        if d.to_string() != "2*x" {
            return Err(format!("CAS self-test failed: d/dx x^2 = {d}"));
        }
        Ok(())
    }

    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self.provider.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocalc_modules::ModuleRegistry;

    fn eval(name: &str, args: &[f64]) -> Result<f64, String> {
        let p = CasProvider::new();
        let nums: Vec<Number> = args.iter().map(|v| Number::from_f64(*v)).collect();
        p.call(name, &nums)
            .unwrap_or(Err(format!("{name} not provided")))
            .map(|n| n.to_f64())
    }

    #[test]
    fn provider_dispatch() {
        // poly_eval(2; 1,2,3) = 1 + 4 + 12 = 17.
        assert_eq!(eval("poly_eval", &[2.0, 1.0, 2.0, 3.0]).unwrap(), 17.0);
        assert_eq!(eval("solve_linear", &[2.0, -4.0]).unwrap(), 2.0);
        assert_eq!(eval("quadratic_disc", &[1.0, 0.0, -4.0]).unwrap(), 16.0);
        assert_eq!(
            eval("quadratic_root", &[1.0, 0.0, -4.0, 1.0]).unwrap(),
            -2.0
        );
        assert_eq!(eval("quadratic_root", &[1.0, 0.0, -4.0, 2.0]).unwrap(), 2.0);
        assert!(eval("quadratic_root", &[1.0, 0.0, 1.0, 1.0]).is_err());
        assert!(eval("quadratic_root", &[1.0, 0.0, -4.0, 3.0]).is_err());
        assert!(eval("solve_linear", &[1.0]).is_err());
        assert!(eval("nope", &[]).is_err());
        // Case-insensitive through the registry path convention.
        assert_eq!(eval("POLY_EVAL", &[2.0, 1.0, 1.0]).unwrap(), 3.0);
    }

    #[test]
    fn registry_loading() {
        let mut r = ModuleRegistry::new();
        r.register(Arc::new(CasModule::new())).unwrap();
        assert!(r.provides_function("solve_linear"));
        let v = r.call_function(
            "solve_linear",
            &[Number::from_f64(2.0), Number::from_f64(-4.0)],
        );
        assert_eq!(v.unwrap().to_f64(), 2.0);
        let meta = CasModule::new().metadata();
        meta.validate().unwrap();
        meta.check_compatibility(ocalc_modules::ENGINE_PLUGIN_API_VERSION)
            .unwrap();
    }
}
