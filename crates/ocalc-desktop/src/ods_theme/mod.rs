//! ODS design system: colors, spacing, typography, shared components.
//!
//! This is the single source of truth for the desktop look-and-feel. Views
//! read tokens from [`OdsTheme`] on every frame; nothing hardcodes sizes or
//! colors except the defaults in this module. `crate::theme` re-exports the
//! stable subset so existing imports keep working.

mod colors;
mod components;
mod spacing;
mod typography;

pub use colors::{OdsColors, ThemeMode, ACCENT_PRESETS};
pub use components::{danger_button, primary_button, subtle_button};
pub use spacing::Spacing;
pub use typography::Typography;

/// Full theme: mode + colors + spacing + typography.
#[derive(Debug, Clone, PartialEq)]
pub struct OdsTheme {
    /// Dark/light/high-contrast selection.
    pub mode: ThemeMode,
    /// Accent/status colors.
    pub colors: OdsColors,
    /// Spacing scale.
    pub spacing: Spacing,
    /// Type scale.
    pub typography: Typography,
    /// Global font-size multiplier (accessibility, 0.875–1.5).
    pub font_scale: f32,
}

impl Default for OdsTheme {
    fn default() -> Self {
        let mode = ThemeMode::Dark;
        Self {
            mode,
            colors: OdsColors::for_mode(mode),
            spacing: Spacing::default(),
            typography: Typography::default(),
            font_scale: 1.0,
        }
    }
}

impl OdsTheme {
    /// New theme for `mode` (resets colors to the mode defaults).
    #[must_use]
    pub fn with_mode(mode: ThemeMode) -> Self {
        Self {
            mode,
            colors: OdsColors::for_mode(mode),
            spacing: Spacing::default(),
            typography: Typography::default(),
            font_scale: 1.0,
        }
    }

    /// Switch mode, resetting accent colors to the mode defaults.
    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;
        self.colors = OdsColors::for_mode(mode);
    }

    /// Override the accent color (settings preset or custom).
    pub fn set_accent(&mut self, accent: iced::Color) {
        self.colors.accent = accent;
    }

    /// Set the global font scale, clamped to the accessible range.
    pub fn set_font_scale(&mut self, scale: f32) {
        self.font_scale = scale.clamp(0.875, 1.5);
    }

    /// Scaled type scale for the current settings.
    #[must_use]
    pub fn scaled_typography(&self) -> Typography {
        self.typography.scaled(self.font_scale)
    }

    /// Map to the Iced theme for the application.
    #[must_use]
    pub fn to_iced(&self) -> iced::Theme {
        self.mode.to_iced()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_and_scale() {
        let mut t = OdsTheme::default();
        t.set_accent(iced::Color::BLACK);
        assert_eq!(t.colors.accent, iced::Color::BLACK);
        t.set_font_scale(99.0);
        assert_eq!(t.font_scale, 1.5);
        t.set_font_scale(0.0);
        assert_eq!(t.font_scale, 0.875);
        let ty = t.scaled_typography();
        assert!((ty.body - Typography::default().body * 0.875).abs() < 1e-6);
    }
}
