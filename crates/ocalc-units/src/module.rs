//! `units` plugin module: exposes the built-in converter as a [`CalculatorModule`].
//!
//! Register [`UnitsModule`] with a [`ModuleRegistry`](ocalc_modules::ModuleRegistry)
//! to make length/mass/temperature/data/energy/time conversions available to
//! hosts through the uniform module interface — no core-engine edits needed.

use ocalc_modules::{CalculatorModule, Capability, ModuleMetadata, UnitProvider};

use crate::CustomUnitRegistry;

/// `units` module (id `"units"`, capabilities `Units + Pure`).
///
/// Wraps a [`CustomUnitRegistry`] seeded with [`crate::builtin_converter`];
/// extra units can be added before registration via [`UnitsModule::registry_mut`].
#[derive(Debug, Clone)]
pub struct UnitsModule {
    registry: CustomUnitRegistry,
}

impl Default for UnitsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitsModule {
    /// New module with all built-in units.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: CustomUnitRegistry::new(),
        }
    }

    /// Borrow the inner registry (read-only).
    #[must_use]
    pub fn registry(&self) -> &CustomUnitRegistry {
        &self.registry
    }

    /// Borrow the inner registry mutably (register custom units pre-`register`).
    pub fn registry_mut(&mut self) -> &mut CustomUnitRegistry {
        &mut self.registry
    }
}

impl UnitProvider for UnitsModule {
    fn unit_names(&self) -> Vec<String> {
        self.registry.converter().symbols()
    }

    fn convert(&self, value: f64, from: &str, to: &str) -> Option<Result<f64, String>> {
        // Claim the pair only when both endpoints are known; otherwise let
        // the next provider try.
        if self.registry.converter().contains(from) && self.registry.converter().contains(to) {
            Some(
                self.registry
                    .convert(value, from, to)
                    .map_err(|e| e.to_string()),
            )
        } else {
            None
        }
    }
}

impl CalculatorModule for UnitsModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "units",
            env!("CARGO_PKG_VERSION"),
            "length/mass/temperature/data/energy/time",
        )
        .with_capability(Capability::Units)
        .with_capability(Capability::Pure)
    }

    fn as_unit_provider(&self) -> Option<&dyn UnitProvider> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocalc_modules::ModuleRegistry;
    use std::sync::Arc;

    #[test]
    fn converts_builtin_units() {
        let m = UnitsModule::new();
        assert!(m.unit_names().contains(&"km".to_owned()));
        assert_eq!(m.convert(1.0, "km", "m").unwrap().unwrap(), 1000.0);
        assert!((m.convert(32.0, "F", "C").unwrap().unwrap()).abs() < 1e-9);
        // Unknown endpoints → None (not handled).
        assert!(m.convert(1.0, "km", "nope").is_none());
    }

    #[test]
    fn dispatches_through_registry() {
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(UnitsModule::new())).unwrap();
        assert!(reg.provides_unit("km"));
        assert!(!reg.provides_unit("smoot"));
        assert_eq!(reg.convert_unit(1.0, "km", "m").unwrap(), 1000.0);
        assert!(reg.convert_unit(1.0, "m", "kg").is_err()); // incompatible
    }

    #[test]
    fn custom_units_extend_the_module() {
        let mut m = UnitsModule::new();
        m.registry_mut()
            .register_custom("smoot", &["smoots"], "m", 1.7018)
            .unwrap();
        assert!(m.unit_names().contains(&"smoot".to_owned()));
        let v = m.convert(1.0, "smoot", "m").unwrap().unwrap();
        assert!((v - 1.7018).abs() < 1e-9);
    }
}
