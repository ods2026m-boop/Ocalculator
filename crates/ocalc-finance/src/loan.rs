//! Loan / amortization calculations (rates as decimal fractions).

/// Monthly payment for a fully amortizing loan.
///
/// `principal` in currency units, `annual_rate` as a decimal (0.06 = 6 % APR),
/// `months` as a positive integer count.
///
/// Standard formula `P·r·(1+r)^n / ((1+r)^n − 1)` with `r = annual_rate / 12`;
/// a zero rate degrades to `principal / months`.
///
/// Example: `loan_payment(10000, 0.06, 12) ≈ 860.66`.
///
/// # Errors
/// Non-finite inputs, `months <= 0` (or non-integer), `principal < 0`,
/// or a degenerate denominator.
pub fn loan_payment(principal: f64, annual_rate: f64, months: f64) -> Result<f64, String> {
    if !principal.is_finite() || !annual_rate.is_finite() || !months.is_finite() {
        return Err("loan_payment: inputs must be finite".to_owned());
    }
    if principal < 0.0 {
        return Err("loan_payment: principal must be non-negative".to_owned());
    }
    if months <= 0.0 || months.fract() != 0.0 {
        return Err("loan_payment: months must be a positive integer".to_owned());
    }
    let n = months;
    let r = annual_rate / 12.0;
    if r == 0.0 {
        return Ok(principal / n);
    }
    if !r.is_finite() || r <= -1.0 {
        return Err("loan_payment: annual rate out of range".to_owned());
    }
    let acc = (1.0 + r).powf(n);
    let denom = acc - 1.0;
    if denom == 0.0 || !denom.is_finite() || !acc.is_finite() {
        return Err("loan_payment: cannot amortize with these parameters".to_owned());
    }
    Ok(principal * r * acc / denom)
}

/// Total amount paid over the life of the loan (`payment × months`).
///
/// # Errors
/// Whatever [`loan_payment`] rejects.
pub fn loan_total(principal: f64, annual_rate: f64, months: f64) -> Result<f64, String> {
    Ok(loan_payment(principal, annual_rate, months)? * months)
}

/// Total interest paid (`total − principal`).
///
/// # Errors
/// Whatever [`loan_payment`] rejects.
pub fn loan_interest(principal: f64, annual_rate: f64, months: f64) -> Result<f64, String> {
    Ok(loan_total(principal, annual_rate, months)? - principal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_values() {
        // $10k at 6 % APR over 12 months ≈ $860.66/mo.
        let p = loan_payment(10_000.0, 0.06, 12.0).unwrap();
        assert!((p - 860.66).abs() < 0.01, "got {p}");
        // Zero rate → straight division.
        assert_eq!(loan_payment(1200.0, 0.0, 12.0).unwrap(), 100.0);
        assert!(loan_payment(1000.0, 0.05, 0.0).is_err());
        assert!(loan_payment(1000.0, 0.05, 12.5).is_err());
        assert!(loan_payment(-100.0, 0.05, 12.0).is_err());
        assert!(loan_payment(f64::INFINITY, 0.05, 12.0).is_err());
    }

    #[test]
    fn totals_agree() {
        let total = loan_total(10_000.0, 0.06, 12.0).unwrap();
        let pmt = loan_payment(10_000.0, 0.06, 12.0).unwrap();
        assert!((total - pmt * 12.0).abs() < 1e-9);
        let interest = loan_interest(10_000.0, 0.06, 12.0).unwrap();
        assert!((interest - (total - 10_000.0)).abs() < 1e-9);
        // Zero-rate loan accrues no interest.
        assert_eq!(loan_interest(1200.0, 0.0, 12.0).unwrap(), 0.0);
    }
}
