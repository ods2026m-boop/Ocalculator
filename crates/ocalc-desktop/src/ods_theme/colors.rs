//! Color tokens: theme modes, palettes, accent presets.

use iced::Color;
use serde::{Deserialize, Serialize};

/// Dark, light, or high-contrast appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Dark background, light text (default).
    #[default]
    Dark,
    /// Light background, dark text.
    Light,
    /// Maximum-contrast dark theme (accessibility).
    HighContrast,
}

impl ThemeMode {
    /// All modes, for pick-lists / segmented controls.
    pub const ALL: [Self; 3] = [Self::Dark, Self::Light, Self::HighContrast];

    /// Human label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::HighContrast => "High contrast",
        }
    }

    /// Map to the Iced built-in theme.
    #[must_use]
    pub fn to_iced(self) -> iced::Theme {
        match self {
            Self::Dark => iced::Theme::Dark,
            Self::Light => iced::Theme::Light,
            // Oxocarbon is the highest-contrast dark built-in: near-black
            // surfaces with bright accents and text.
            Self::HighContrast => iced::Theme::Oxocarbon,
        }
    }
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Configurable accent/status colors.
///
/// Base surfaces come from the Iced theme; these tokens color the display,
/// keypad accents, graph curves and status text. All are runtime-editable
/// (settings keep the struct; views read it on every frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OdsColors {
    /// Primary accent (evaluate button, active mode, graph curve).
    pub accent: Color,
    /// Success / valid-graph status.
    pub success: Color,
    /// Error text / invalid-graph status.
    pub danger: Color,
    /// Muted secondary text (history timestamps, hints).
    pub muted: Color,
}

impl OdsColors {
    /// Default palette tuned for the dark theme.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            accent: Color::from_rgb8(0x4C, 0x8D, 0xFF),
            success: Color::from_rgb8(0x3F, 0xB6, 0x6B),
            danger: Color::from_rgb8(0xE5, 0x48, 0x4D),
            muted: Color::from_rgb8(0x8B, 0x93, 0xA3),
        }
    }

    /// Default palette tuned for the light theme.
    #[must_use]
    pub fn light() -> Self {
        Self {
            accent: Color::from_rgb8(0x1F, 0x6F, 0xEB),
            success: Color::from_rgb8(0x1A, 0x7F, 0x4B),
            danger: Color::from_rgb8(0xCF, 0x22, 0x2E),
            muted: Color::from_rgb8(0x5F, 0x6B, 0x7A),
        }
    }

    /// Palette tuned for maximum contrast (pure accents on near-black).
    #[must_use]
    pub fn high_contrast() -> Self {
        Self {
            accent: Color::from_rgb8(0x33, 0xBB, 0xFF),
            success: Color::from_rgb8(0x00, 0xFF, 0x99),
            danger: Color::from_rgb8(0xFF, 0x55, 0x55),
            muted: Color::from_rgb8(0xCC, 0xCC, 0xCC),
        }
    }

    /// Palette matching `mode`.
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::HighContrast => Self::high_contrast(),
        }
    }
}

/// Curated accent presets offered by the settings panel.
pub const ACCENT_PRESETS: [Color; 5] = [
    Color::from_rgb8(0x4C, 0x8D, 0xFF), // ODS blue
    Color::from_rgb8(0x3F, 0xB6, 0x6B), // green
    Color::from_rgb8(0x9D, 0x6B, 0xFF), // violet
    Color::from_rgb8(0xFF, 0x9E, 0x2C), // orange
    Color::from_rgb8(0xE5, 0x48, 0x4D), // red
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_roundtrip() {
        assert_eq!(ThemeMode::Dark.label(), "Dark");
        assert!(matches!(ThemeMode::Dark.to_iced(), iced::Theme::Dark));
        assert!(matches!(ThemeMode::Light.to_iced(), iced::Theme::Light));
        assert!(matches!(
            ThemeMode::HighContrast.to_iced(),
            iced::Theme::Oxocarbon
        ));
        assert_eq!(ThemeMode::ALL.len(), 3);
        assert_eq!(
            OdsColors::for_mode(ThemeMode::HighContrast),
            OdsColors::high_contrast()
        );
    }
}
