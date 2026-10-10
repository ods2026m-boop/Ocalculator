//! Core converter: categories, definitions, linear + affine conversion.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::UnitError;

/// Physical category. Conversions are only valid within one category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitCategory {
    /// Length / distance.
    Length,
    /// Mass / weight.
    Mass,
    /// Temperature (affine).
    Temperature,
    /// Digital data.
    Data,
    /// Energy.
    Energy,
    /// Time / duration.
    Time,
    /// User-registered category.
    Custom(&'static str),
}

impl std::fmt::Display for UnitCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Length => write!(f, "length"),
            Self::Mass => write!(f, "mass"),
            Self::Temperature => write!(f, "temperature"),
            Self::Data => write!(f, "data"),
            Self::Energy => write!(f, "energy"),
            Self::Time => write!(f, "time"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

/// Definition of a single unit.
#[derive(Debug, Clone)]
pub struct UnitDefinition {
    /// Canonical symbol (`"km"`).
    pub symbol: &'static str,
    /// Aliases (`["kilometer", "kilometers"]`).
    pub aliases: &'static [&'static str],
    /// Category.
    pub category: UnitCategory,
    /// Multiply by this to reach the base unit of the category.
    pub factor_to_base: f64,
    /// Added *after* scaling when going to base (0 for linear units;
    /// used for temperature offsets).
    pub offset_to_base: f64,
}

impl UnitDefinition {
    /// Linear unit helper.
    pub const fn linear(
        symbol: &'static str,
        aliases: &'static [&'static str],
        category: UnitCategory,
        factor_to_base: f64,
    ) -> Self {
        Self {
            symbol,
            aliases,
            category,
            factor_to_base,
            offset_to_base: 0.0,
        }
    }
}

/// Converts values between units of the same category.
#[derive(Debug, Default, Clone)]
pub struct UnitConverter {
    by_symbol: HashMap<String, UnitDefinition>,
}

impl UnitConverter {
    /// Empty converter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_symbol: HashMap::new(),
        }
    }

    /// Register a unit plus all its aliases (case-insensitive).
    pub fn register(&mut self, def: UnitDefinition) {
        for key in std::iter::once(def.symbol).chain(def.aliases.iter().copied()) {
            self.by_symbol.insert(key.to_ascii_lowercase(), def.clone());
        }
    }

    /// Look up a definition (case-insensitive).
    #[must_use]
    pub fn lookup(&self, symbol: &str) -> Option<&UnitDefinition> {
        self.by_symbol.get(&symbol.to_ascii_lowercase())
    }

    /// True when `symbol` is known.
    #[must_use]
    pub fn contains(&self, symbol: &str) -> bool {
        self.lookup(symbol).is_some()
    }

    /// Convert `value` from `from` to `to`.
    ///
    /// # Errors
    /// - [`UnitError::UnknownUnit`] for unregistered symbols.
    /// - [`UnitError::Incompatible`] across categories.
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Result<f64, UnitError> {
        let a = self
            .lookup(from)
            .ok_or_else(|| UnitError::UnknownUnit(from.to_owned()))?;
        let b = self
            .lookup(to)
            .ok_or_else(|| UnitError::UnknownUnit(to.to_owned()))?;
        if a.category != b.category {
            return Err(UnitError::Incompatible {
                from: from.to_owned(),
                from_cat: a.category.to_string(),
                to: to.to_owned(),
                to_cat: b.category.to_string(),
            });
        }
        // to_base = value * factor + offset; from_base = (to_base - off_b)/fac_b
        let base = value * a.factor_to_base + a.offset_to_base;
        Ok((base - b.offset_to_base) / b.factor_to_base)
    }

    /// All canonical symbols (sorted, deduplicated).
    #[must_use]
    pub fn symbols(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .by_symbol
            .values()
            .map(|d| d.symbol.to_owned())
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin_converter;

    #[test]
    fn case_insensitive_lookup() {
        let c = builtin_converter();
        assert!(c.contains("KM"));
        assert!(c.contains("kilometer"));
    }

    #[test]
    fn incompatible_errors() {
        let c = builtin_converter();
        let e = c.convert(1.0, "m", "kg").unwrap_err();
        assert!(matches!(e, UnitError::Incompatible { .. }));
    }
}
