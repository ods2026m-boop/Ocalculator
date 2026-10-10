//! Percentage calculations.

/// `percent(base, pct)` — `pct` percent of `base`.
///
/// Example: `percent(200, 15) = 30`.
///
/// # Errors
/// Non-finite inputs.
pub fn percent(base: f64, pct: f64) -> Result<f64, String> {
    if !base.is_finite() || !pct.is_finite() {
        return Err("percent: inputs must be finite".to_owned());
    }
    Ok(base * pct / 100.0)
}

/// `percent_change(old, new)` — percentage change from `old` to `new`.
///
/// Example: `percent_change(100, 120) = 20`.
///
/// # Errors
/// `old == 0` (undefined) or non-finite inputs.
pub fn percent_change(old: f64, new: f64) -> Result<f64, String> {
    if !old.is_finite() || !new.is_finite() {
        return Err("percent_change: inputs must be finite".to_owned());
    }
    if old == 0.0 {
        return Err("percent_change: old value must be non-zero".to_owned());
    }
    Ok((new - old) / old * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_values() {
        assert_eq!(percent(200.0, 15.0).unwrap(), 30.0);
        assert_eq!(percent(80.0, 100.0).unwrap(), 80.0);
        assert_eq!(percent(50.0, 0.0).unwrap(), 0.0);
        assert!(percent(f64::INFINITY, 1.0).is_err());
        assert!(percent(1.0, f64::NAN).is_err());
    }

    #[test]
    fn change_values() {
        assert_eq!(percent_change(100.0, 120.0).unwrap(), 20.0);
        assert_eq!(percent_change(200.0, 100.0).unwrap(), -50.0);
        assert_eq!(percent_change(50.0, 50.0).unwrap(), 0.0);
        assert!(percent_change(0.0, 5.0).is_err());
        assert!(percent_change(f64::INFINITY, 1.0).is_err());
    }
}
