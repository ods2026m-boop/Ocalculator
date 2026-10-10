//! Spacing scale (logical pixels). One scale for the whole app.

/// Spacing tokens (logical pixels, as `f32` for direct use with
/// Iced layout primitives).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// Tight gaps (inside keypads).
    pub xs: f32,
    /// Default gaps (rows/columns).
    pub sm: f32,
    /// Panel padding.
    pub md: f32,
    /// Section separation.
    pub lg: f32,
    /// Display font size (kept here for back-compat; prefer [`Typography`](super::Typography)).
    pub display_text: f32,
    /// Keypad button text size (kept here for back-compat).
    pub button_text: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 20.0,
            display_text: 32.0,
            button_text: 18.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_sane() {
        let s = Spacing::default();
        assert!(s.xs <= s.sm && s.sm <= s.md && s.md <= s.lg);
    }
}
