//! `chemistry` plugin module: numeric bridge into the engine.
//!
//! Formula strings (`H2O`, `Ca(OH)2`) live in the native Rust API
//! ([`parse_formula`](crate::parse_formula),
//! [`molecular_mass`](crate::molecular_mass)) — engine calls carry
//! [`Number`](ocalc_number::Number), never strings. This provider exposes the
//! numeric kernels instead: `element_mass` and `molar_mass`.

use std::sync::Arc;

use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

use crate::{molar_mass_pairs, table};

/// Provider names (lowercase, sorted).
const FUNCTIONS: [&str; 2] = ["element_mass", "molar_mass"];

/// Numeric chemistry kernels as a [`FunctionProvider`].
#[derive(Debug, Clone, Default)]
pub struct ChemistryProvider;

impl ChemistryProvider {
    /// Shared instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn dispatch(name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        match name {
            "element_mass" => Some(do_element_mass(args)),
            "molar_mass" => Some(do_molar_mass(args)),
            _ => None,
        }
    }
}

impl FunctionProvider for ChemistryProvider {
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

/// `element_mass(z)` — atomic weight (g/mol) of atomic number `z`.
fn do_element_mass(args: &[Number]) -> Result<Number, String> {
    if args.len() != 1 {
        return Err("element_mass(z) needs one atomic number".to_owned());
    }
    let z = args[0].to_f64();
    if z.fract() != 0.0 || z < 1.0 || z > 118.0 {
        return Err(format!(
            "element_mass: atomic number must be 1–118, got {z}"
        ));
    }
    table::by_number(z as u32)
        .map(|e| Number::from_f64(e.mass))
        .ok_or_else(|| format!("unknown atomic number {z}"))
}

/// `molar_mass(z1, n1, z2, n2, …)` — molar mass (g/mol) from `(atomic number,
/// count)` pairs, e.g. water as `molar_mass(1, 2, 8, 1)` = 18.015.
fn do_molar_mass(args: &[Number]) -> Result<Number, String> {
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err("molar_mass(z1, n1, …) needs even (atomic number, count) pairs".to_owned());
    }
    let mut pairs = Vec::with_capacity(args.len() / 2);
    for chunk in args.chunks(2) {
        let (z, n) = (chunk[0].to_f64(), chunk[1].to_f64());
        if z.fract() != 0.0 || z < 1.0 || z > 118.0 {
            return Err(format!("molar_mass: atomic number must be 1–118, got {z}"));
        }
        if n.fract() != 0.0 || n < 1.0 {
            return Err(format!(
                "molar_mass: count must be a positive integer, got {n}"
            ));
        }
        pairs.push((z as u32, n as u32));
    }
    molar_mass_pairs(&pairs).map(Number::from_f64)
}

/// `chemistry` calculator module.
#[derive(Debug, Clone)]
pub struct ChemistryModule {
    provider: Arc<ChemistryProvider>,
}

impl Default for ChemistryModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ChemistryModule {
    /// New module (runs a native-API self-test on [`init`](CalculatorModule::init)).
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: Arc::new(ChemistryProvider::new()),
        }
    }
}

impl CalculatorModule for ChemistryModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "chemistry",
            env!("CARGO_PKG_VERSION"),
            "chemistry toolkit: periodic table, formula parser, molar masses",
        )
        .with_capability(Capability::Custom("chemistry".into()))
    }

    fn init(&self) -> Result<(), String> {
        // Native self-test: H2O must weigh 18.015 g/mol.
        let m =
            crate::molecular_mass("H2O").map_err(|e| format!("chemistry self-test failed: {e}"))?;
        if (m - 18.015).abs() > 1e-9 {
            return Err(format!("chemistry self-test failed: H2O = {m}"));
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
        let p = ChemistryProvider::new();
        let nums: Vec<Number> = args.iter().map(|v| Number::from_f64(*v)).collect();
        p.call(name, &nums)
            .unwrap_or(Err(format!("{name} not provided")))
            .map(|n| n.to_f64())
    }

    #[test]
    fn provider_dispatch() {
        assert_eq!(eval("element_mass", &[1.0]).unwrap(), 1.008);
        assert_eq!(eval("element_mass", &[8.0]).unwrap(), 15.999);
        // molar_mass(1,2, 8,1) = H2O.
        assert!((eval("molar_mass", &[1.0, 2.0, 8.0, 1.0]).unwrap() - 18.015).abs() < 1e-9);
        // molar_mass(6,1, 8,2) = CO2.
        assert!((eval("molar_mass", &[6.0, 1.0, 8.0, 2.0]).unwrap() - 44.009).abs() < 1e-9);
        assert!(eval("element_mass", &[0.0]).is_err());
        assert!(eval("element_mass", &[119.0]).is_err());
        assert!(eval("molar_mass", &[1.0]).is_err());
        assert!(eval("molar_mass", &[]).is_err());
        assert!(eval("nope", &[]).is_err());
        assert_eq!(eval("ELEMENT_MASS", &[1.0]).unwrap(), 1.008);
    }

    #[test]
    fn registry_loading() {
        let mut r = ModuleRegistry::new();
        r.register(Arc::new(ChemistryModule::new())).unwrap();
        assert!(r.provides_function("molar_mass"));
        let v = r.call_function("element_mass", &[Number::from_f64(6.0)]);
        assert_eq!(v.unwrap().to_f64(), 12.011);
        let meta = ChemistryModule::new().metadata();
        meta.validate().unwrap();
        meta.check_compatibility(ocalc_modules::ENGINE_PLUGIN_API_VERSION)
            .unwrap();
    }
}
