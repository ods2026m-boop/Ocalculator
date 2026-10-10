//! Physical constants (exact SI definitions where fixed).

/// Standard gravity `g = 9.80665 m/s²`.
pub const GRAVITY: f64 = 9.80665;

/// Speed of light `c = 299 792 458 m/s` (exact).
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Planck constant `h = 6.62607015e-34 J·s` (exact).
pub const PLANCK: f64 = 6.626_070_15e-34;

/// Elementary charge `e = 1.602176634e-19 C` (exact).
pub const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;

/// Avogadro constant `Nₐ = 6.02214076e23 mol⁻¹` (exact).
pub const AVOGADRO: f64 = 6.022_140_76e23;

/// Boltzmann constant `k = 1.380649e-23 J/K` (exact).
pub const BOLTZMANN: f64 = 1.380_649e-23;

/// All exported constants as `(function-name, value, unit)`.
pub const ALL: [(&str, f64, &str); 6] = [
    ("gravity", GRAVITY, "m/s²"),
    ("speed_of_light", SPEED_OF_LIGHT, "m/s"),
    ("planck", PLANCK, "J·s"),
    ("elementary_charge", ELEMENTARY_CHARGE, "C"),
    ("avogadro", AVOGADRO, "mol⁻¹"),
    ("boltzmann", BOLTZMANN, "J/K"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn si_values() {
        assert_eq!(GRAVITY, 9.80665);
        assert_eq!(SPEED_OF_LIGHT, 299_792_458.0);
        assert!(PLANCK > 0.0 && PLANCK < 1e-30);
        assert_eq!(ALL.len(), 6);
    }
}
