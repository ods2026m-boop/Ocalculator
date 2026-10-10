//! Chemical formula parser.
//!
//! Parses formulas like `H2O`, `CO2`, `NaCl`, `Ca(OH)2`, `Al2(SO4)3` into
//! element counts. Groups use `()` or `[]` with an optional trailing
//! multiplier; whitespace is ignored. Hydrate dots (`·`, `.`) split the
//! formula into independently parsed parts whose counts are merged
//! (`CuSO4·5H2O` works, with an optional leading multiplier on each part).

use std::collections::BTreeMap;

use crate::table::lookup;

/// Element counts for one parsed formula: symbol → atom count.
pub type Composition = BTreeMap<String, u32>;

/// Parse `formula` into element counts.
///
/// # Errors
/// Empty input, unknown element symbols, malformed groups or counts.
pub fn parse_formula(formula: &str) -> Result<Composition, String> {
    let no_space: String = formula.chars().filter(|c| !c.is_whitespace()).collect();
    if no_space.is_empty() {
        return Err("empty formula".to_owned());
    }
    let mut total: Composition = BTreeMap::new();
    // Hydrate separator: middle dot or plain dot between parts.
    for part in no_space.split(['·', '.']) {
        if part.is_empty() {
            return Err(format!("malformed formula {formula:?}"));
        }
        let (mult, rest) = leading_count(part);
        let counts = parse_group(rest, formula)?;
        for (sym, n) in counts {
            *total.entry(sym).or_insert(0) += n * mult;
        }
    }
    Ok(total)
}

/// Leading `5H2O`-style multiplier (defaults to 1).
fn leading_count(s: &str) -> (u32, &str) {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        (1, s)
    } else {
        let n: u32 = digits.parse().unwrap_or(1);
        (n.max(1), &s[digits.len()..])
    }
}

/// Parse one dot-separated part (no leading multiplier, no dots).
fn parse_group(s: &str, full: &str) -> Result<Composition, String> {
    // Stack of open groups; the bottom is the formula root.
    let mut stack: Vec<Composition> = vec![BTreeMap::new()];
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '(' || c == '[' {
            stack.push(BTreeMap::new());
            i += 1;
        } else if c == ')' || c == ']' {
            if stack.len() < 2 {
                return Err(format!("unmatched {c:?} in {full:?}"));
            }
            i += 1;
            let (mult, rest) = read_count(&s[i..]);
            i = s.len() - rest.len();
            let group = stack.pop().unwrap_or_default();
            let top = stack.last_mut().unwrap_or_else(|| unreachable!());
            for (sym, n) in group {
                *top.entry(sym).or_insert(0) += n * mult;
            }
        } else if c.is_ascii_uppercase() {
            let mut sym = String::from(c);
            i += 1;
            if i < bytes.len() && (bytes[i] as char).is_ascii_lowercase() {
                sym.push(bytes[i] as char);
                i += 1;
            }
            if lookup(&sym).is_none() {
                return Err(format!("unknown element {sym:?} in {full:?}"));
            }
            let (mult, rest) = read_count(&s[i..]);
            i = s.len() - rest.len();
            *stack
                .last_mut()
                .unwrap_or_else(|| unreachable!())
                .entry(sym)
                .or_insert(0) += mult;
        } else {
            return Err(format!("unexpected {c:?} in {full:?}"));
        }
    }
    if stack.len() != 1 {
        return Err(format!("unclosed group in {full:?}"));
    }
    let root = stack.pop().unwrap_or_default();
    if root.is_empty() {
        return Err(format!("malformed formula {full:?}"));
    }
    Ok(root)
}

/// Trailing integer count (defaults to 1). A leading `0` is rejected.
fn read_count(s: &str) -> (u32, &str) {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.starts_with('0') {
        (1, s)
    } else {
        (digits.parse().unwrap_or(1), &s[digits.len()..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(m: &Composition, sym: &str) -> u32 {
        m.get(sym).copied().unwrap_or(0)
    }

    #[test]
    fn simple_formulas() {
        let h2o = parse_formula("H2O").unwrap();
        assert_eq!((get(&h2o, "H"), get(&h2o, "O")), (2, 1));
        let co2 = parse_formula("CO2").unwrap();
        assert_eq!((get(&co2, "C"), get(&co2, "O")), (1, 2));
        let nacl = parse_formula("NaCl").unwrap();
        assert_eq!((get(&nacl, "Na"), get(&nacl, "Cl")), (1, 1));
        let glucose = parse_formula("C6H12O6").unwrap();
        assert_eq!(
            (get(&glucose, "C"), get(&glucose, "H"), get(&glucose, "O")),
            (6, 12, 6)
        );
    }

    #[test]
    fn grouped_formulas() {
        // Ca(OH)2 → Ca + 2×(O,H).
        let slaked = parse_formula("Ca(OH)2").unwrap();
        assert_eq!(
            (get(&slaked, "Ca"), get(&slaked, "O"), get(&slaked, "H")),
            (1, 2, 2)
        );
        // Al2(SO4)3 → 2 Al, 3 S, 12 O.
        let alum = parse_formula("Al2(SO4)3").unwrap();
        assert_eq!(
            (get(&alum, "Al"), get(&alum, "S"), get(&alum, "O")),
            (2, 3, 12)
        );
        // Brackets behave like parens.
        let same = parse_formula("Al2[SO4]3").unwrap();
        assert_eq!(same, alum);
    }

    #[test]
    fn hydrates_and_spacing() {
        let blue = parse_formula("CuSO4·5H2O").unwrap();
        assert_eq!(
            (
                get(&blue, "Cu"),
                get(&blue, "S"),
                get(&blue, "O"),
                get(&blue, "H")
            ),
            (1, 1, 9, 10)
        );
        let spaced = parse_formula(" H2O ").unwrap();
        assert_eq!(get(&spaced, "H"), 2);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_formula("").is_err());
        assert!(parse_formula("Xx2").is_err());
        assert!(parse_formula("H2O)").is_err());
        assert!(parse_formula("(H2O").is_err());
        assert!(parse_formula("2").is_err());
        assert!(parse_formula("h2o").is_err());
    }
}
