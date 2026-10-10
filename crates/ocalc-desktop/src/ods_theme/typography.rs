//! Typography scale + accessibility font scaling.

/// Type sizes in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    /// Result display.
    pub display: f32,
    /// Panel headings.
    pub heading: f32,
    /// Body / keypad labels.
    pub body: f32,
    /// Hints, timestamps, status bar.
    pub small: f32,
    /// Dense readouts (history rows, bit view).
    pub tiny: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            display: 32.0,
            heading: 20.0,
            body: 16.0,
            small: 13.0,
            tiny: 12.0,
        }
    }
}

impl Typography {
    /// Scale every size by `scale` (font-size accessibility setting).
    #[must_use]
    pub fn scaled(self, scale: f32) -> Self {
        Self {
            display: self.display * scale,
            heading: self.heading * scale,
            body: self.body * scale,
            small: self.small * scale,
            tiny: self.tiny * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_orders() {
        let t = Typography::default();
        assert!(
            t.tiny <= t.small && t.small <= t.body && t.body <= t.heading && t.heading <= t.display
        );
        let big = t.scaled(1.5);
        assert!((big.body - t.body * 1.5).abs() < 1e-6);
    }
}
