//! Growth calculations (rates as decimal fractions).

/// `growth(start, rate, periods)` — compounded growth.
///
/// `start * (1 + rate)^periods`.
///
/// Example: `growth(100, 0.1, 2) = 121`.
///
/// # Errors
/// Non-finite inputs, negative periods, or a non-positive base with a
/// fractional exponent.
pub fn growth(start: f64, rate: f64, periods: f64) -> Result<f64, String> {
    if !start.is_finite() || !rate.is_finite() || !periods.is_finite() {
        return Err("growth: inputs must be finite".to_owned());
    }
    if periods < 0.0 {
        return Err("growth: periods must be non-negative".to_owned());
    }
    let base = 1.0 + rate;
    if base <= 0.0 && periods.fract() != 0.0 {
        return Err("growth: (1 + rate) must be positive for fractional periods".to_owned());
    }
    Ok(start * base.powf(periods))
}

/// `growth_rate(start, end, periods)` — per-period rate taking `start` to
/// `end` in `periods` steps: `(end/start)^(1/periods) − 1`.
///
/// Example: `growth_rate(100, 121, 2) = 0.1` (10 %).
///
/// # Errors
/// `start <= 0`, `end < 0`, or `periods <= 0`.
pub fn growth_rate(start: f64, end: f64, periods: f64) -> Result<f64, String> {
    if !start.is_finite() || !end.is_finite() || !periods.is_finite() {
        return Err("growth_rate: inputs must be finite".to_owned());
    }
    if start <= 0.0 {
        return Err("growth_rate: start must be positive".to_owned());
    }
    if end < 0.0 {
        return Err("growth_rate: end must be non-negative".to_owned());
    }
    if periods <= 0.0 {
        return Err("growth_rate: periods must be positive".to_owned());
    }
    Ok((end / start).powf(1.0 / periods) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_values() {
        assert!((growth(100.0, 0.1, 2.0).unwrap() - 121.0).abs() < 1e-9);
        assert_eq!(growth(100.0, 0.0, 5.0).unwrap(), 100.0);
        assert!(growth(100.0, 0.1, -1.0).is_err());
        assert!(growth(f64::NAN, 0.1, 1.0).is_err());
    }

    #[test]
    fn rate_roundtrip() {
        assert!((growth_rate(100.0, 121.0, 2.0).unwrap() - 0.1).abs() < 1e-12);
        // growth(growth_rate(...)) round-trips.
        let r = growth_rate(1000.0, 2000.0, 10.0).unwrap();
        assert!((growth(1000.0, r, 10.0).unwrap() - 2000.0).abs() < 1e-6);
        assert!(growth_rate(0.0, 100.0, 2.0).is_err());
        assert!(growth_rate(100.0, 100.0, 0.0).is_err());
    }
}
