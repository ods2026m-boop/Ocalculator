//! Electricity in SI units (V, A, Ω, W).

/// Voltage `V = I·R` (Ohm's law).
pub fn voltage(current_a: f64, resistance_ohm: f64) -> f64 {
    current_a * resistance_ohm
}

/// Current `I = V / R`.
///
/// # Errors
/// Zero resistance.
pub fn current(voltage_v: f64, resistance_ohm: f64) -> Result<f64, String> {
    if resistance_ohm == 0.0 {
        return Err("current: resistance must be non-zero".to_owned());
    }
    Ok(voltage_v / resistance_ohm)
}

/// Resistance `R = V / I`.
///
/// # Errors
/// Zero current.
pub fn resistance(voltage_v: f64, current_a: f64) -> Result<f64, String> {
    if current_a == 0.0 {
        return Err("resistance: current must be non-zero".to_owned());
    }
    Ok(voltage_v / current_a)
}

/// Power `P = V·I` (W).
pub fn power_vi(voltage_v: f64, current_a: f64) -> f64 {
    voltage_v * current_a
}

/// Power `P = I²·R` (W).
pub fn power_ri(current_a: f64, resistance_ohm: f64) -> f64 {
    current_a * current_a * resistance_ohm
}

/// Power `P = V²/R` (W).
///
/// # Errors
/// Zero resistance.
pub fn power_vr(voltage_v: f64, resistance_ohm: f64) -> Result<f64, String> {
    if resistance_ohm == 0.0 {
        return Err("power: resistance must be non-zero".to_owned());
    }
    Ok(voltage_v * voltage_v / resistance_ohm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ohm_and_power() {
        assert_eq!(voltage(2.0, 3.0), 6.0);
        assert_eq!(current(6.0, 3.0).unwrap(), 2.0);
        assert_eq!(resistance(6.0, 2.0).unwrap(), 3.0);
        assert_eq!(power_vi(6.0, 2.0), 12.0);
        assert_eq!(power_ri(2.0, 3.0), 12.0);
        assert_eq!(power_vr(6.0, 3.0).unwrap(), 12.0);
        assert!(current(1.0, 0.0).is_err());
        assert!(resistance(1.0, 0.0).is_err());
        assert!(power_vr(1.0, 0.0).is_err());
    }
}
