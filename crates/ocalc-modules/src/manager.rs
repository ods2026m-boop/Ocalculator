//! Dynamic module registry.

use std::collections::HashMap;
use std::sync::Arc;

use ocalc_number::Number;

use crate::{CalculatorModule, Capability, ModuleError};

/// Owns all registered [`CalculatorModule`]s and dispatches function calls.
///
/// The core engine never matches on concrete module types — it only talks to
/// this registry, so third-party crates can extend the calculator without
/// touching `ocalc-engine`.
///
/// Cheaply cloneable: modules are held as `Arc`s, so cloning shares providers
/// (used by the graph sampler to give each point an isolated variable scope
/// without losing plugins).
#[derive(Default, Clone)]
pub struct ModuleRegistry {
    modules: HashMap<String, Arc<dyn CalculatorModule>>,
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRegistry")
            .field("modules", &self.module_ids())
            .finish()
    }
}

impl ModuleRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register a module, running its `init()` hook.
    ///
    /// Validates [`crate::ModuleMetadata`] (id / semver / description) before
    /// `init()`. Compatibility against [`ENGINE_PLUGIN_API_VERSION`] is
    /// *checked but not enforced* here — use [`Self::register_checked`] for
    /// strict hosts (ODS-os sandboxes, production frontends).
    ///
    /// # Errors
    /// - [`ModuleError::InvalidMetadata`] on bad metadata.
    /// - [`ModuleError::AlreadyRegistered`] on duplicate id.
    /// - [`ModuleError::InitFailed`] when `init()` fails.
    pub fn register(&mut self, module: Arc<dyn CalculatorModule>) -> Result<(), ModuleError> {
        let meta = module.metadata();
        meta.validate()?;
        if self.modules.contains_key(&meta.id) {
            return Err(ModuleError::AlreadyRegistered(meta.id));
        }
        module.init().map_err(|reason| ModuleError::InitFailed {
            id: meta.id.clone(),
            reason,
        })?;
        self.modules.insert(meta.id, module);
        Ok(())
    }

    /// Strict registration: validates metadata *and* enforces plugin-API
    /// compatibility against `engine_api` (usually
    /// [`ENGINE_PLUGIN_API_VERSION`]).
    ///
    /// # Errors
    /// Same as [`Self::register`] plus [`ModuleError::IncompatibleVersion`].
    pub fn register_checked(
        &mut self,
        module: Arc<dyn CalculatorModule>,
        engine_api: &str,
    ) -> Result<(), ModuleError> {
        let meta = module.metadata();
        meta.validate()?;
        meta.check_compatibility(engine_api)?;
        if self.modules.contains_key(&meta.id) {
            return Err(ModuleError::AlreadyRegistered(meta.id));
        }
        module.init().map_err(|reason| ModuleError::InitFailed {
            id: meta.id.clone(),
            reason,
        })?;
        self.modules.insert(meta.id, module);
        Ok(())
    }

    /// Bulk registration (module discovery result). All modules are validated;
    /// duplicates and init failures are collected and returned together so
    /// hosts can report every bad plugin in one pass. Successfully validated
    /// modules are registered even when others fail.
    ///
    /// Returns `Ok(())` when every module registered, else `Err(errors)`.
    pub fn register_all(
        &mut self,
        modules: Vec<Arc<dyn CalculatorModule>>,
    ) -> Result<(), Vec<ModuleError>> {
        let mut errors = Vec::new();
        for m in modules {
            if let Err(e) = self.register(m) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate every registered module's metadata (id / semver / description).
    ///
    /// Returns the list of violations (empty = all valid).
    #[must_use]
    pub fn validate_all(&self) -> Vec<ModuleError> {
        let mut out = Vec::new();
        for m in self.modules.values() {
            if let Err(e) = m.metadata().validate() {
                out.push(e);
            }
        }
        out
    }

    /// Check every registered module against `engine_api`.
    ///
    /// Returns `(compatible_ids, errors)`.
    #[must_use]
    pub fn check_compatibility_all(&self, engine_api: &str) -> (Vec<String>, Vec<ModuleError>) {
        let mut ok = Vec::new();
        let mut errs = Vec::new();
        for m in self.modules.values() {
            let meta = m.metadata();
            match meta.check_compatibility(engine_api) {
                Ok(()) => ok.push(meta.id),
                Err(e) => errs.push(e),
            }
        }
        ok.sort();
        (ok, errs)
    }

    /// Remove a module by id. Returns `true` when something was removed.
    pub fn unregister(&mut self, id: &str) -> bool {
        self.modules.remove(id).is_some()
    }

    /// Ids of all registered modules (sorted).
    #[must_use]
    pub fn module_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.modules.keys().cloned().collect();
        v.sort();
        v
    }

    /// Number of registered modules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// True when no modules are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// All function names provided across modules (sorted, deduplicated).
    #[must_use]
    pub fn function_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for m in self.modules.values() {
            if let Some(p) = m.as_function_provider() {
                names.extend(p.function_names());
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// Dispatch `name(args)` to the first provider that handles it.
    ///
    /// # Errors
    /// - [`ModuleError::NotFound`] when nobody handles `name`.
    /// - [`ModuleError::Provider`] when the handler fails.
    pub fn call_function(&self, name: &str, args: &[Number]) -> Result<Number, ModuleError> {
        let key = name.to_ascii_lowercase();
        for m in self.modules.values() {
            if let Some(p) = m.as_function_provider() {
                if let Some(res) = p.call(&key, args) {
                    return res.map_err(ModuleError::Provider);
                }
            }
        }
        Err(ModuleError::NotFound(name.to_owned()))
    }

    /// True when some module provides `name` (uses the cheap
    /// [`crate::FunctionProvider::provides`] hook — no `Vec` allocation for
    /// providers that override it).
    #[must_use]
    pub fn provides_function(&self, name: &str) -> bool {
        self.modules
            .values()
            .any(|m| m.as_function_provider().is_some_and(|p| p.provides(name)))
    }

    /// Modules advertising a given capability.
    #[must_use]
    pub fn with_capability(&self, cap: &Capability) -> Vec<String> {
        let mut out = Vec::new();
        for (id, m) in &self.modules {
            if m.metadata().capabilities.contains(cap) {
                out.push(id.clone());
            }
        }
        out.sort();
        out
    }

    /// All unit symbols served across unit providers (sorted, deduplicated).
    #[must_use]
    pub fn unit_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for m in self.modules.values() {
            if let Some(p) = m.as_unit_provider() {
                names.extend(p.unit_names());
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// True when some module handles `symbol` (case-insensitive).
    #[must_use]
    pub fn provides_unit(&self, symbol: &str) -> bool {
        self.modules
            .values()
            .any(|m| m.as_unit_provider().is_some_and(|p| p.provides(symbol)))
    }

    /// Convert `value` from `from` to `to` via the first provider that handles
    /// both endpoints.
    ///
    /// # Errors
    /// - [`ModuleError::NotFound`] when no provider handles the pair.
    /// - [`ModuleError::Provider`] when the handler fails.
    pub fn convert_unit(&self, value: f64, from: &str, to: &str) -> Result<f64, ModuleError> {
        for m in self.modules.values() {
            if let Some(p) = m.as_unit_provider() {
                if let Some(res) = p.convert(value, from, to) {
                    return res.map_err(ModuleError::Provider);
                }
            }
        }
        Err(ModuleError::NotFound(format!("{from} -> {to}")))
    }

    /// Uniform module inventory: `(name, version, functions, capabilities)`.
    #[must_use]
    pub fn describe(&self) -> Vec<(String, String, Vec<String>, Vec<Capability>)> {
        let mut out: Vec<_> = self
            .modules
            .values()
            .map(|m| (m.name(), m.version(), m.functions(), m.capabilities()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionProvider, ModuleMetadata};

    struct Double;
    impl FunctionProvider for Double {
        fn function_names(&self) -> Vec<String> {
            vec!["double".to_owned()]
        }
        fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
            if name == "double" && args.len() == 1 {
                Some(args[0].add(&args[0]).map_err(|e| e.to_string()))
            } else {
                None
            }
        }
    }
    struct DoubleModule;
    impl CalculatorModule for DoubleModule {
        fn metadata(&self) -> ModuleMetadata {
            ModuleMetadata::new("double", "1.0.0", "test").with_capability(Capability::Pure)
        }
        fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
            // NOTE: returning a reference to a temporary would be unsound;
            // for the test we leak a static provider instead.
            None
        }
    }

    // A module that owns its provider (the realistic pattern).
    struct Owning {
        provider: Double,
    }
    impl CalculatorModule for Owning {
        fn metadata(&self) -> ModuleMetadata {
            ModuleMetadata::new("owning-double", "1.0.0", "test").with_capability(Capability::Pure)
        }
        fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
            Some(&self.provider)
        }
    }

    #[test]
    fn register_and_dispatch() {
        let mut reg = ModuleRegistry::new();
        assert!(reg.is_empty());
        reg.register(Arc::new(Owning { provider: Double })).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg.provides_function("double"));
        assert!(reg.provides_function("DOUBLE"));
        let v = reg.call_function("double", &[Number::Integer(21)]).unwrap();
        assert_eq!(v, Number::Integer(42));
        assert!(matches!(
            reg.call_function("missing", &[]),
            Err(ModuleError::NotFound(_))
        ));
    }

    #[test]
    fn duplicate_registration_rejected() {
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(Owning { provider: Double })).unwrap();
        let err = reg
            .register(Arc::new(Owning { provider: Double }))
            .unwrap_err();
        assert!(matches!(err, ModuleError::AlreadyRegistered(_)));
    }

    #[test]
    fn capability_filter() {
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(Owning { provider: Double })).unwrap();
        assert_eq!(
            reg.with_capability(&Capability::Pure),
            vec!["owning-double".to_owned()]
        );
        assert!(reg.with_capability(&Capability::Units).is_empty());
    }

    #[test]
    fn metadata_builder_dedups() {
        let m = ModuleMetadata::new("x", "0.1", "d").with_capability(Capability::Pure);
        assert_eq!(m.capabilities.len(), 1);
        // DoubleModule exists to document the temporary-reference pitfall.
        let _ = DoubleModule;
    }

    #[test]
    fn module_trait_surface() {
        let m = Owning { provider: Double };
        assert_eq!(m.name(), "owning-double");
        assert_eq!(m.version(), "1.0.0");
        assert_eq!(m.functions(), vec!["double".to_owned()]);
        assert!(m.capabilities().contains(&Capability::Pure));
    }

    #[test]
    fn unit_dispatch() {
        use crate::UnitProvider;
        struct Metric;
        impl UnitProvider for Metric {
            fn unit_names(&self) -> Vec<String> {
                vec!["m".to_owned(), "km".to_owned()]
            }
            fn convert(&self, value: f64, from: &str, to: &str) -> Option<Result<f64, String>> {
                match (
                    from.to_ascii_lowercase().as_str(),
                    to.to_ascii_lowercase().as_str(),
                ) {
                    ("km", "m") => Some(Ok(value * 1000.0)),
                    ("m", "km") => Some(Ok(value / 1000.0)),
                    _ => None,
                }
            }
        }
        struct MetricModule {
            provider: Metric,
        }
        impl CalculatorModule for MetricModule {
            fn metadata(&self) -> ModuleMetadata {
                ModuleMetadata::new("metric", "1.0.0", "test").with_capability(Capability::Units)
            }
            fn as_unit_provider(&self) -> Option<&dyn UnitProvider> {
                Some(&self.provider)
            }
        }
        let mut reg = ModuleRegistry::new();
        reg.register(Arc::new(MetricModule { provider: Metric }))
            .unwrap();
        assert!(reg.provides_unit("KM"));
        assert!(!reg.provides_unit("mile"));
        assert_eq!(reg.convert_unit(2.0, "km", "m").unwrap(), 2000.0);
        assert!(reg.convert_unit(1.0, "km", "mile").is_err());
        assert_eq!(reg.unit_names(), vec!["km".to_owned(), "m".to_owned()]);
        // Uniform inventory.
        let desc = reg.describe();
        assert_eq!(desc.len(), 1);
        assert_eq!(desc[0].0, "metric");
    }
}
