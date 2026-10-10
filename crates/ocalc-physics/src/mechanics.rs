//! Classical mechanics in SI units.
//!
//! All functions take SI inputs (kg, m, s, N, J) and return SI outputs.
//! Non-SI inputs convert first through the existing unit system
//! ([`ocalc_units::builtin_converter`]) — see `kmh_to_ms` / `ms_to_kmh`.

use crate::constants::GRAVITY;

/// Velocity `v = d / t` (m/s).
///
/// # Errors
/// Zero time.
pub fn velocity(distance_m: f64, time_s: f64) -> Result<f64, String> {
    if time_s == 0.0 {
        return Err("velocity: time must be non-zero".to_owned());
    }
    Ok(distance_m / time_s)
}

/// Acceleration `a = Δv / t` (m/s²).
///
/// # Errors
/// Zero time.
pub fn acceleration(delta_v: f64, time_s: f64) -> Result<f64, String> {
    if time_s == 0.0 {
        return Err("acceleration: time must be non-zero".to_owned());
    }
    Ok(delta_v / time_s)
}

/// Force `F = m·a` (N).
pub fn force(mass_kg: f64, accel_ms2: f64) -> f64 {
    mass_kg * accel_ms2
}

/// Weight `W = m·g` (N).
pub fn weight(mass_kg: f64) -> f64 {
    mass_kg * GRAVITY
}

/// Momentum `p = m·v` (kg·m/s).
pub fn momentum(mass_kg: f64, velocity_ms: f64) -> f64 {
    mass_kg * velocity_ms
}

/// Kinetic energy `E = ½·m·v²` (J).
///
/// # Errors
/// Non-finite inputs.
pub fn kinetic_energy(mass_kg: f64, velocity_ms: f64) -> Result<f64, String> {
    if !mass_kg.is_finite() || !velocity_ms.is_finite() {
        return Err("kinetic_energy: inputs must be finite".to_owned());
    }
    Ok(0.5 * mass_kg * velocity_ms * velocity_ms)
}

/// Gravitational potential energy `E = m·g·h` (J).
pub fn potential_energy(mass_kg: f64, height_m: f64) -> f64 {
    mass_kg * GRAVITY * height_m
}

/// Convert km/h → m/s through the existing unit tables.
pub fn kmh_to_ms(v: f64) -> f64 {
    // 1 km/h = 1000 m / 3600 s; resolved through the converter so any
    // future table correction propagates automatically.
    let conv = ocalc_units::builtin_converter();
    conv.convert(v, "km", "m").unwrap_or(v * 1000.0) / conv.convert(1.0, "h", "s").unwrap_or(3600.0)
}

/// Convert m/s → km/h through the existing unit tables.
pub fn ms_to_kmh(v: f64) -> f64 {
    let conv = ocalc_units::builtin_converter();
    conv.convert(v, "m", "km").unwrap_or(v / 1000.0)
        / conv.convert(1.0, "s", "h").unwrap_or(1.0 / 3600.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newton_basics() {
        assert_eq!(force(2.0, 3.0), 6.0);
        assert_eq!(weight(1.0), GRAVITY);
        assert_eq!(momentum(2.0, 3.0), 6.0);
        assert_eq!(velocity(100.0, 10.0).unwrap(), 10.0);
        assert_eq!(acceleration(10.0, 2.0).unwrap(), 5.0);
        assert!(velocity(1.0, 0.0).is_err());
        assert!(acceleration(1.0, 0.0).is_err());
    }

    #[test]
    fn energies() {
        assert_eq!(kinetic_energy(2.0, 3.0).unwrap(), 9.0);
        assert_eq!(potential_energy(2.0, 10.0), 2.0 * GRAVITY * 10.0);
        assert!(kinetic_energy(f64::INFINITY, 1.0).is_err());
    }

    #[test]
    fn speed_helpers_use_unit_tables() {
        assert!((kmh_to_ms(36.0) - 10.0).abs() < 1e-9);
        assert!((ms_to_kmh(10.0) - 36.0).abs() < 1e-9);
    }
}
