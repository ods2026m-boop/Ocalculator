//! Descriptive statistics over [`Number`] slices.

use ocalc_number::Number;

use crate::FunctionError;

/// Sum of all arguments.
pub fn sum(args: &[Number]) -> Result<Number, FunctionError> {
    let mut acc = Number::Integer(0);
    for a in args {
        acc = acc.add(a)?;
    }
    Ok(acc)
}

/// Arithmetic mean. Errors on empty input.
pub fn mean(args: &[Number]) -> Result<Number, FunctionError> {
    if args.is_empty() {
        return Err(FunctionError::Domain {
            name: "mean".to_owned(),
            message: "empty input".to_owned(),
        });
    }
    let s = sum(args)?;
    s.div(&Number::from_i64(args.len() as i64))
        .map_err(FunctionError::from)
}

/// Median (middle value, or mean of two middles). Errors on empty input.
pub fn median(args: &[Number]) -> Result<Number, FunctionError> {
    if args.is_empty() {
        return Err(FunctionError::Domain {
            name: "median".to_owned(),
            message: "empty input".to_owned(),
        });
    }
    let mut vals: Vec<f64> = args.iter().map(|n| n.to_f64()).collect();
    vals.sort_by(|a, b| a.total_cmp(b));
    let mid = vals.len() / 2;
    if vals.len() % 2 == 1 {
        Ok(Number::Float(vals[mid]))
    } else {
        Ok(Number::Float((vals[mid - 1] + vals[mid]) / 2.0))
    }
}

/// Population variance (divides by N).
pub fn variance(args: &[Number]) -> Result<Number, FunctionError> {
    if args.is_empty() {
        return Err(FunctionError::Domain {
            name: "variance".to_owned(),
            message: "empty input".to_owned(),
        });
    }
    let m = mean(args)?.to_f64();
    let mut acc = 0.0;
    for a in args {
        let d = a.to_f64() - m;
        acc += d * d;
    }
    Ok(Number::Float(acc / args.len() as f64))
}

/// Population standard deviation.
pub fn stddev(args: &[Number]) -> Result<Number, FunctionError> {
    Ok(Number::Float(variance(args)?.to_f64().sqrt()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_smoke() {
        let v = vec![Number::Float(1.0), Number::Float(2.0), Number::Float(3.0)];
        assert_eq!(sum(&v).unwrap().to_f64(), 6.0);
        assert_eq!(mean(&v).unwrap().to_f64(), 2.0);
        assert_eq!(median(&v).unwrap().to_f64(), 2.0);
        assert!((variance(&v).unwrap().to_f64() - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn empty_errors() {
        assert!(mean(&[]).is_err());
        assert!(median(&[]).is_err());
    }
}
