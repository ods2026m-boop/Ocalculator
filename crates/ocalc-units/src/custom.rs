//! Runtime custom-unit registry (wraps [`UnitConverter`]).

use crate::{UnitConverter, UnitDefinition, UnitError};

/// Owns a [`UnitConverter`] seeded with built-ins plus user units.
///
/// Custom linear units are added via [`CustomUnitRegistry::register_custom`].
#[derive(Debug, Clone)]
pub struct CustomUnitRegistry {
    converter: UnitConverter,
}

impl Default for CustomUnitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomUnitRegistry {
    /// Registry pre-loaded with [`crate::builtin_converter`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            converter: crate::builtin_converter(),
        }
    }

    /// Register a custom linear unit.
    ///
    /// `factor_to_base` is expressed in terms of an existing unit of the
    /// same category: `register_custom("smoot", &["smoots"], "m", 1.7018)`
    /// defines one smoot as 1.7018 m.
    ///
    /// # Errors
    /// [`UnitError::UnknownUnit`] when `base_unit` is unknown.
    pub fn register_custom(
        &mut self,
        symbol: &'static str,
        aliases: &'static [&'static str],
        base_unit: &str,
        factor_in_base: f64,
    ) -> Result<(), UnitError> {
        let base = self
            .converter
            .lookup(base_unit)
            .ok_or_else(|| UnitError::UnknownUnit(base_unit.to_owned()))?;
        let def = UnitDefinition {
            symbol,
            aliases,
            category: base.category,
            factor_to_base: base.factor_to_base * factor_in_base,
            offset_to_base: 0.0,
        };
        // SAFETY: `symbol`/`aliases` are `&'static` by signature.
        self.converter.register(def);
        Ok(())
    }

    /// Convert via the inner converter.
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Result<f64, UnitError> {
        self.converter.convert(value, from, to)
    }

    /// Access the inner converter.
    #[must_use]
    pub fn converter(&self) -> &UnitConverter {
        &self.converter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_unit() {
        let mut r = CustomUnitRegistry::new();
        r.register_custom("smoot", &["smoots"], "m", 1.7018)
            .unwrap();
        let m = r.convert(1.0, "smoot", "m").unwrap();
        assert!((m - 1.7018).abs() < 1e-9);
    }

    #[test]
    fn unknown_base_errors() {
        let mut r = CustomUnitRegistry::new();
        assert!(r.register_custom("x", &[], "nope", 1.0).is_err());
    }
}
