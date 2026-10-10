//! Symbolic variables.
//!
//! A [`Symbol`] is a named placeholder (`x`, `y`, `z`, …) used by
//! [`SymExpr`](crate::SymExpr). Symbols carry no value; [`substitute`](crate::substitute)
//! replaces them with expressions.

/// A symbolic variable name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(String);

impl Symbol {
    /// New symbol. Names must be non-empty ASCII alphanumeric/`_` and are
    /// conventionally single lowercase letters (`x`, `y`, `z`).
    ///
    /// # Errors
    /// Empty or non-identifier names.
    pub fn new(name: &str) -> Result<Self, String> {
        let ok = !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if ok {
            Ok(Self(name.to_owned()))
        } else {
            Err(format!("invalid symbol name {name:?}"))
        }
    }

    /// Variable `x`.
    #[must_use]
    pub fn x() -> Self {
        Self("x".to_owned())
    }

    /// Variable `y`.
    #[must_use]
    pub fn y() -> Self {
        Self("y".to_owned())
    }

    /// Variable `z`.
    #[must_use]
    pub fn z() -> Self {
        Self("z".to_owned())
    }

    /// The name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_and_invalid_names() {
        assert_eq!(Symbol::new("x").unwrap().name(), "x");
        assert_eq!(Symbol::new("alpha_1").unwrap().name(), "alpha_1");
        assert!(Symbol::new("").is_err());
        assert!(Symbol::new("1x").is_err());
        assert!(Symbol::new("x+y").is_err());
        assert_eq!(Symbol::x().to_string(), "x");
    }
}
