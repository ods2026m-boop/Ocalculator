//! `finance` plugin module: percent / interest / loan / growth as engine functions.

use std::sync::Arc;

use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

use crate::{growth as native_growth, interest, loan, percent as native_percent};

/// Provider names (lowercase, sorted).
const FUNCTIONS: [&str; 9] = [
    "compound",
    "effective_rate",
    "growth",
    "growth_rate",
    "loan_interest",
    "loan_payment",
    "loan_total",
    "percent",
    "percent_change",
];

/// Finance kernels as a [`FunctionProvider`] (rates are decimal fractions).
#[derive(Debug, Clone, Default)]
pub struct FinanceProvider;

impl FinanceProvider {
    /// Shared instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl FunctionProvider for FinanceProvider {
    fn function_names(&self) -> Vec<String> {
        FUNCTIONS.iter().map(|s| (*s).to_owned()).collect()
    }

    fn provides(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        FUNCTIONS.iter().any(|f| *f == key)
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        let key = name.to_ascii_lowercase();
        let v: Vec<f64> = args.iter().map(|n| n.to_f64()).collect();
        let need = |n: usize, usage: &str| {
            if v.len() != n {
                Some(Err(usage.to_owned()))
            } else {
                None
            }
        };
        match key.as_str() {
            "percent" => need(2, "percent(base, pct)")
                .or_else(|| Some(native_percent::percent(v[0], v[1]).map(Number::from_f64))),
            "percent_change" => need(2, "percent_change(old, new)")
                .or_else(|| Some(native_percent::percent_change(v[0], v[1]).map(Number::from_f64))),
            "compound" => need(3, "compound(principal, rate, periods)")
                .or_else(|| Some(interest::compound(v[0], v[1], v[2]).map(Number::from_f64))),
            "effective_rate" => need(2, "effective_rate(nominal, compounds_per_year)")
                .or_else(|| Some(interest::effective_rate(v[0], v[1]).map(Number::from_f64))),
            "loan_payment" => need(3, "loan_payment(principal, annual_rate, months)")
                .or_else(|| Some(loan::loan_payment(v[0], v[1], v[2]).map(Number::from_f64))),
            "loan_total" => need(3, "loan_total(principal, annual_rate, months)")
                .or_else(|| Some(loan::loan_total(v[0], v[1], v[2]).map(Number::from_f64))),
            "loan_interest" => need(3, "loan_interest(principal, annual_rate, months)")
                .or_else(|| Some(loan::loan_interest(v[0], v[1], v[2]).map(Number::from_f64))),
            "growth" => need(3, "growth(start, rate, periods)")
                .or_else(|| Some(native_growth::growth(v[0], v[1], v[2]).map(Number::from_f64))),
            "growth_rate" => need(3, "growth_rate(start, end, periods)").or_else(|| {
                Some(native_growth::growth_rate(v[0], v[1], v[2]).map(Number::from_f64))
            }),
            _ => None,
        }
    }
}

/// `finance` calculator module.
#[derive(Debug, Clone)]
pub struct FinanceModule {
    provider: Arc<FinanceProvider>,
}

impl Default for FinanceModule {
    fn default() -> Self {
        Self::new()
    }
}

impl FinanceModule {
    /// New module.
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: Arc::new(FinanceProvider::new()),
        }
    }
}

impl CalculatorModule for FinanceModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "finance",
            env!("CARGO_PKG_VERSION"),
            "finance toolkit: percent, compound interest, loans, growth",
        )
        .with_capability(Capability::Custom("finance".into()))
    }

    fn init(&self) -> Result<(), String> {
        // Self-test: 15 % of 200 is 30; 1k at 5 % for 1 period is 1050.
        if native_percent::percent(200.0, 15.0).map_err(|e| format!("finance self-test: {e}"))?
            != 30.0
        {
            return Err("finance self-test failed: percent".to_owned());
        }
        let c =
            interest::compound(1000.0, 0.05, 1.0).map_err(|e| format!("finance self-test: {e}"))?;
        if (c - 1050.0).abs() > 1e-9 {
            return Err(format!("finance self-test failed: compound = {c}"));
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
        let p = FinanceProvider::new();
        let nums: Vec<Number> = args.iter().map(|v| Number::from_f64(*v)).collect();
        p.call(name, &nums)
            .unwrap_or(Err(format!("{name} not provided")))
            .map(|n| n.to_f64())
    }

    #[test]
    fn provider_dispatch() {
        assert_eq!(eval("percent", &[200.0, 15.0]).unwrap(), 30.0);
        assert_eq!(eval("percent_change", &[100.0, 120.0]).unwrap(), 20.0);
        assert!((eval("compound", &[1000.0, 0.05, 1.0]).unwrap() - 1050.0).abs() < 1e-9);
        let pmt = eval("loan_payment", &[10_000.0, 0.06, 12.0]).unwrap();
        assert!((pmt - 860.66).abs() < 0.01, "got {pmt}");
        assert!((eval("growth", &[100.0, 0.1, 2.0]).unwrap() - 121.0).abs() < 1e-9);
        assert!((eval("growth_rate", &[100.0, 121.0, 2.0]).unwrap() - 0.1).abs() < 1e-12);
        assert!(eval("percent", &[1.0]).is_err());
        assert!(eval("compound", &[1.0, 0.05]).is_err());
        assert!(eval("loan_payment", &[1000.0, 0.05, 0.0]).is_err());
        assert!(eval("nope", &[]).is_err());
        assert_eq!(eval("PERCENT", &[200.0, 50.0]).unwrap(), 100.0);
    }

    #[test]
    fn registry_loading() {
        let mut r = ModuleRegistry::new();
        r.register(Arc::new(FinanceModule::new())).unwrap();
        assert!(r.provides_function("compound"));
        assert!(r.provides_function("loan_payment"));
        let v = r.call_function(
            "percent",
            &[Number::from_f64(200.0), Number::from_f64(15.0)],
        );
        assert_eq!(v.unwrap().to_f64(), 30.0);
        let meta = FinanceModule::new().metadata();
        meta.validate().unwrap();
        meta.check_compatibility(ocalc_modules::ENGINE_PLUGIN_API_VERSION)
            .unwrap();
    }
}
