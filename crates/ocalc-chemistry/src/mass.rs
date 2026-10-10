//! Molecular weights and composition from parsed formulas.

use crate::formula::{parse_formula, Composition};
use crate::table::{lookup, ElementData};

/// One element's contribution to a compound.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// The element.
    pub element: ElementData,
    /// Atom count in the formula.
    pub count: u32,
    /// `count × atomic weight` (g/mol).
    pub mass: f64,
    /// Mass fraction in percent.
    pub percent: f64,
}

/// Molar mass of `formula` in g/mol (`H2O` → 18.015).
///
/// # Errors
/// Whatever [`parse_formula`] rejects.
pub fn molecular_mass(formula: &str) -> Result<f64, String> {
    Ok(composition(formula)?.iter().map(|c| c.mass).sum())
}

/// Per-element breakdown of `formula` with mass percents.
///
/// # Errors
/// Whatever [`parse_formula`] rejects.
pub fn composition(formula: &str) -> Result<Vec<Component>, String> {
    let counts: Composition = parse_formula(formula)?;
    let mut parts: Vec<Component> = Vec::with_capacity(counts.len());
    for (sym, count) in &counts {
        let element =
            lookup(sym).ok_or_else(|| format!("unknown element {sym:?} in {formula:?}"))?;
        parts.push(Component {
            element,
            count: *count,
            mass: f64::from(*count) * element.mass,
            percent: 0.0,
        });
    }
    let total: f64 = parts.iter().map(|c| c.mass).sum();
    if total > 0.0 {
        for c in &mut parts {
            c.percent = 100.0 * c.mass / total;
        }
    }
    parts.sort_by_key(|a| a.element.number);
    Ok(parts)
}

/// Molar mass from `(atomic number, count)` pairs, e.g. water as
/// `[(1, 2), (8, 1)]`. This is the numeric kernel behind the engine-callable
/// `molar_mass` function (engine calls carry [`Number`](ocalc_number::Number),
/// never formula strings).
///
/// # Errors
/// Odd-length input, unknown atomic numbers, or non-positive counts.
pub fn molar_mass_pairs(pairs: &[(u32, u32)]) -> Result<f64, String> {
    if pairs.is_empty() {
        return Err("molar_mass needs at least one (atomic number, count) pair".to_owned());
    }
    let mut total = 0.0;
    for (z, n) in pairs {
        let e = crate::table::by_number(*z).ok_or_else(|| format!("unknown atomic number {z}"))?;
        if *n == 0 {
            return Err(format!("count for {} must be positive", e.symbol));
        }
        total += f64::from(*n) * e.mass;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_masses() {
        // H2O = 2×1.008 + 15.999.
        assert!((molecular_mass("H2O").unwrap() - 18.015).abs() < 1e-9);
        // CO2 = 12.011 + 2×15.999.
        assert!((molecular_mass("CO2").unwrap() - 44.009).abs() < 1e-9);
        // NaCl = 22.990 + 35.45.
        assert!((molecular_mass("NaCl").unwrap() - 58.44).abs() < 1e-9);
        // H2SO4 = 2×1.008 + 32.06 + 4×15.999.
        assert!((molecular_mass("H2SO4").unwrap() - 98.072).abs() < 1e-9);
    }

    #[test]
    fn percents_sum_to_100() {
        let parts = composition("H2O").unwrap();
        assert_eq!(parts.len(), 2);
        let sum: f64 = parts.iter().map(|c| c.percent).sum();
        assert!((sum - 100.0).abs() < 1e-9);
        assert_eq!(parts[0].element.symbol, "H");
    }

    #[test]
    fn numeric_pairs_match_formulas() {
        // H2O as (Z, n) pairs.
        let m = molar_mass_pairs(&[(1, 2), (8, 1)]).unwrap();
        assert!((m - molecular_mass("H2O").unwrap()).abs() < 1e-12);
        assert!(molar_mass_pairs(&[]).is_err());
        assert!(molar_mass_pairs(&[(0, 1)]).is_err());
        assert!(molar_mass_pairs(&[(1, 0)]).is_err());
    }

    #[test]
    fn bad_formula_propagates() {
        assert!(molecular_mass("Xx2").is_err());
        assert!(molecular_mass("").is_err());
    }
}
