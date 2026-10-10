//! Built-in function modules: each category as a [`CalculatorModule`].
//!
//! The engine never hardcodes function names. It either calls the merged
//! [`FunctionRegistry`](crate::FunctionRegistry) (angle-aware fast path) or
//! dispatches through [`ModuleRegistry`](ocalc_modules::ModuleRegistry) to
//! these modules (plugin path). Both paths serve the same implementations.
//!
//! Available modules:
//! - [`StandardMathModule`] (`standard-math`): rounding, min/max, integers.
//! - [`ScientificModule`] (`scientific-math`): trig, logs, exp, roots.
//! - [`StatisticsModule`] (`statistics`): sum, mean, median, variance.
//!
//! NOTE on angle modes: [`FunctionProvider`](ocalc_modules::FunctionProvider)
//! has no angle context, so trig through the *module* path evaluates in
//! radians. The engine's primary `FunctionRegistry` path is angle-aware and
//! must be preferred for `sin`/`cos`/… when degrees matter.

use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

use crate::{FunctionContext, FunctionRegistry};

// ---------------------------------------------------------------------------
// Shared provider shim: FunctionRegistry → FunctionProvider
// ---------------------------------------------------------------------------

/// Adapts a [`FunctionRegistry`] to [`FunctionProvider`] (radians context).
#[derive(Debug, Clone)]
struct RegistryProvider {
    registry: FunctionRegistry,
}

impl RegistryProvider {
    fn new(registry: FunctionRegistry) -> Self {
        Self { registry }
    }
}

impl FunctionProvider for RegistryProvider {
    fn function_names(&self) -> Vec<String> {
        self.registry.names()
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        let key = name.to_ascii_lowercase();
        if !self.registry.contains(&key) {
            return None;
        }
        // Module path has no angle context → radians (documented above).
        let ctx = FunctionContext::default();
        Some(
            self.registry
                .call(&key, args, &ctx)
                .map_err(|e| e.to_string()),
        )
    }
}

// ---------------------------------------------------------------------------
// standard-math
// ---------------------------------------------------------------------------

/// `standard-math` module: rounding, min/max/clamp, integer helpers.
#[derive(Debug, Clone)]
pub struct StandardMathModule {
    provider: RegistryProvider,
}

impl Default for StandardMathModule {
    fn default() -> Self {
        Self::new()
    }
}

impl StandardMathModule {
    /// New module with a fresh [`crate::standard_registry`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: RegistryProvider::new(crate::standard_registry()),
        }
    }
}

impl FunctionProvider for StandardMathModule {
    fn function_names(&self) -> Vec<String> {
        self.provider.function_names()
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        self.provider.call(name, args)
    }
}

impl CalculatorModule for StandardMathModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "standard-math",
            env!("CARGO_PKG_VERSION"),
            "rounding, min/max, gcd/lcm, factorial",
        )
        .with_capability(Capability::Pure)
    }

    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self)
    }
}

// ---------------------------------------------------------------------------
// scientific-math
// ---------------------------------------------------------------------------

/// `scientific-math` module: trig, logs, exp, roots, hypot.
#[derive(Debug, Clone)]
pub struct ScientificModule {
    provider: RegistryProvider,
}

impl Default for ScientificModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ScientificModule {
    /// New module with a fresh [`crate::scientific_registry`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: RegistryProvider::new(crate::scientific_registry()),
        }
    }
}

impl FunctionProvider for ScientificModule {
    fn function_names(&self) -> Vec<String> {
        self.provider.function_names()
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        self.provider.call(name, args)
    }
}

impl CalculatorModule for ScientificModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "scientific-math",
            env!("CARGO_PKG_VERSION"),
            "trigonometry, logarithms, exponentials, roots (radians on module path)",
        )
        .with_capability(Capability::Pure)
        .with_capability(Capability::Trigonometry)
    }

    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self)
    }
}

// ---------------------------------------------------------------------------
// statistics
// ---------------------------------------------------------------------------

/// `statistics` module: sum, mean/avg, median, variance, stddev.
#[derive(Debug, Clone)]
pub struct StatisticsModule {
    provider: RegistryProvider,
}

impl Default for StatisticsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl StatisticsModule {
    /// New module with a fresh [`crate::statistics_registry`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: RegistryProvider::new(crate::statistics_registry()),
        }
    }
}

impl FunctionProvider for StatisticsModule {
    fn function_names(&self) -> Vec<String> {
        self.provider.function_names()
    }

    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        self.provider.call(name, args)
    }
}

impl CalculatorModule for StatisticsModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "statistics",
            env!("CARGO_PKG_VERSION"),
            "sum, mean, median, variance, stddev",
        )
        .with_capability(Capability::Pure)
        .with_capability(Capability::Statistics)
    }

    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_partition_the_default_registry() {
        let (std, sci, stats) = (
            crate::standard_registry(),
            crate::scientific_registry(),
            crate::statistics_registry(),
        );
        assert_eq!(
            std.len() + sci.len() + stats.len(),
            crate::default_registry().len()
        );
        // No overlap between categories.
        for n in std.names() {
            assert!(!sci.contains(&n), "{n} in two categories");
            assert!(!stats.contains(&n), "{n} in two categories");
        }
    }

    #[test]
    fn modules_serve_their_functions() {
        let m = StandardMathModule::new();
        assert!(m.function_names().contains(&"gcd".to_owned()));
        assert!(m
            .call("gcd", &[Number::Integer(12), Number::Integer(18)])
            .unwrap()
            .is_ok());
        assert!(m.call("sin", &[]).is_none());

        let s = ScientificModule::new();
        assert!(s.call("sqrt", &[Number::Float(9.0)]).unwrap().is_ok());
        // Module path is radians-based and case-insensitive.
        let v = s.call("SIN", &[Number::Float(0.0)]).unwrap().unwrap();
        assert_eq!(v.to_f64(), 0.0);

        let st = StatisticsModule::new();
        let v = st
            .call(
                "mean",
                &[Number::Float(1.0), Number::Float(2.0), Number::Float(3.0)],
            )
            .unwrap()
            .unwrap();
        assert_eq!(v.to_f64(), 2.0);
        assert!(st.call("nope", &[]).is_none());
    }

    #[test]
    fn metadata_advertises_capabilities() {
        use ocalc_modules::Capability;
        assert!(StandardMathModule::new()
            .metadata()
            .capabilities
            .contains(&Capability::Pure));
        assert!(ScientificModule::new()
            .metadata()
            .capabilities
            .contains(&Capability::Trigonometry));
        assert!(StatisticsModule::new()
            .metadata()
            .capabilities
            .contains(&Capability::Statistics));
    }
}
