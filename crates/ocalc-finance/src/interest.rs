//! Compound-interest calculations (rate as a decimal fraction).

/// `compound(principal, rate, periods)` — future value with compounding.
///
/// `principal * (1 + rate)^periods`; `rate = 0.05` means 5 % per period.
///
/// Example: `compound(1000, 0.05, 1) = 1050`.
///
/// # Errors
/// Non-finite inputs, negative periods, or `1 + rate <= 0` with a
/// non-integer exponent (would be complex / undefined in reals).
pub fn compound(principal: f64, rate: f64, periods: f64) -> Result<f64, String> {
    if !principal.is_finite() || !rate.is_finite() || !periods.is_finite() {
        return Err("compound: inputs must be finite".to_owned());
    }
    if periods < 0.0 {
        return Err("compound: periods must be non-negative".to_owned());
    }
    let base = 1.0 + rate;
    if base <= 0.0 && periods.fract() != 0.0 {
        return Err("compound: (1 + rate) must be positive for fractional periods".to_owned());
    }
    Ok(principal * base.powf(periods))
}

/// Effective annual rate from a nominal rate compounded `m` times per year.
///
/// `nominal` is a decimal fraction (0.12 = 12 %).
///
/// # Errors
/// Non-finite inputs, `m <= 0`, or a non-positive base.
pub fn effective_rate(nominal: f64, m: f64) -> Result<f64, String> {
    if !nominal.is_finite() || !m.is_finite() {
        return Err("effective_rate: inputs must be finite".to_owned());
    }
    if m <= 0.0 || m.fract() != 0.0 {
        return Err("effective_rate: compounds per year must be a positive integer".to_owned());
    }
    let base = 1.0 + nominal / m;
    if base <= 0.0 {
        return Err("effective_rate: (1 + nominal/m) must be positive".to_owned());
    }
    Ok(base.powf(m) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_values() {
        assert!((compound(1000.0, 0.05, 1.0).unwrap() - 1050.0).abs() < 1e-9);
        assert!((compound(1000.0, 0.05, 10.0).unwrap() - 1628.894626777442).abs() < 1e-6);
        assert_eq!(compound(500.0, 0.0, 12.0).unwrap(), 500.0);
        assert_eq!(compound(500.0, 0.1, 0.0).unwrap(), 500.0);
        assert!(compound(100.0, 0.05, -1.0).is_err());
        assert!(compound(f64::INFINITY, 0.05, 1.0).is_err());
        assert!(compound(100.0, -2.0, 0.5).is_err());
    }

    #[test]
    fn effective_annual() {
        // 12 % nominal monthly → ≈ 12.6825 % effective.
        let e = effective_rate(0.12, 12.0).unwrap();
        assert!((e - 0.126_825_030_131_969_77).abs() < 1e-12);
        assert!(effective_rate(0.05, 0.0).is_err());
        assert!(effective_rate(0.05, 1.5).is_err());
    }
}
