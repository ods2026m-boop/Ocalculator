//! Engine commands: the **only** place that calls OCalculator APIs.
//!
//! Views and `app.rs` never touch the engine directly — they call these
//! helpers, which wrap the stable API (`CalculatorEngine`, `Expression`,
//! `Value`, `builtin_converter`, graph sampling). This guarantees no
//! duplicated calculation logic: every mode formats the same `Value`.

use std::collections::HashSet;

use ocalc_engine::{CalculatorEngine, GraphPoint, Value};

/// Maximum display precision (decimal places) offered by settings.
pub const MAX_PRECISION: u8 = 12;

/// Outcome of evaluating one input line.
#[derive(Debug, Clone)]
pub struct EvalOutcome {
    /// Engine result, if successful.
    pub result: Option<Value>,
    /// Display string (formatted result or empty on error).
    ///
    /// Production `AppState::submit` re-renders through the active
    /// `NumberFormat` (standard vs scientific), so it uses `result` + its own
    /// formatter; `display` remains the canonical standard-precision rendering
    /// for headless callers and tests.
    #[allow(dead_code)]
    pub display: String,
    /// Human error message, if failed.
    pub error: Option<String>,
}

/// Evaluate `src` with `engine` and format via `precision`.
///
/// Never panics: empty input yields an empty outcome; engine errors become
/// `error` strings (with caret rendering when spans exist).
pub fn evaluate(engine: &mut CalculatorEngine, src: &str, precision: u8) -> EvalOutcome {
    let src = src.trim();
    if src.is_empty() {
        return EvalOutcome {
            result: None,
            display: String::new(),
            error: None,
        };
    }
    match engine.eval(src) {
        Ok(v) => {
            let display = format_value(&v, precision);
            EvalOutcome {
                result: Some(v),
                display,
                error: None,
            }
        }
        Err(e) => EvalOutcome {
            result: None,
            display: String::new(),
            error: Some(e.render_with_source(src)),
        },
    }
}

/// Format a [`Value`] for the display with `precision` decimal places.
///
/// Integer-like variants (`Integer`/`BigInt`/`Rational`) and `Complex` keep
/// exact `to_string()` output regardless of precision. `Float`/`Decimal` are
/// rounded to `precision` places (trailing zeros trimmed; `precision >= 12`
/// means full exact display).
pub fn format_value(value: &Value, precision: u8) -> String {
    format_number(value, precision, false)
}

/// Format a [`Value`] like [`format_value`], with optional scientific
/// notation for `Float`/`Decimal` results (`1.23e10`). Exact variants
/// (`Integer`/`BigInt`/`Rational`/`Complex`) are never touched.
pub fn format_number(value: &Value, precision: u8, scientific: bool) -> String {
    if !scientific {
        return match value {
            Value::Float(f) => format_float(*f, precision),
            Value::Decimal(d) => {
                if precision >= MAX_PRECISION {
                    d.to_string()
                } else {
                    format_float(d.to_string().parse::<f64>().unwrap_or(f64::NAN), precision)
                }
            }
            _ => value.to_string(),
        };
    }
    match value {
        Value::Float(f) => format_scientific(*f, precision),
        Value::Decimal(d) => {
            if precision >= MAX_PRECISION {
                d.to_string()
            } else {
                format_scientific(d.to_string().parse::<f64>().unwrap_or(f64::NAN), precision)
            }
        }
        _ => value.to_string(),
    }
}

/// Scientific notation with a trimmed mantissa (`15000 → 1.5e4`;
/// `precision` caps mantissa fraction digits, minimum 1).
fn format_scientific(f: f64, precision: u8) -> String {
    if !f.is_finite() {
        return Value::Float(f).to_string();
    }
    if f == 0.0 {
        return "0".to_owned();
    }
    let p = (precision as usize).min(MAX_PRECISION as usize).max(1);
    let s = format!("{f:.p$e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((s.as_str(), "0"));
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    format!("{mantissa}e{exp}")
}

fn format_float(f: f64, precision: u8) -> String {
    if !f.is_finite() {
        return Value::Float(f).to_string();
    }
    if precision >= MAX_PRECISION {
        return Value::Float(f).to_string();
    }
    let p = precision as usize;
    let s = format!("{f:.p$}");
    // Trim trailing zeros but keep at least one digit after the point when the
    // caller asked for fractional places... actually for a calculator display
    // `2.50` → `2.5`, `3.00` → `3`.
    if s.contains('.') {
        let t = s.trim_end_matches('0').trim_end_matches('.');
        if t.is_empty() || t == "-0" {
            "0".to_owned()
        } else {
            t.to_owned()
        }
    } else {
        s
    }
}

/// Convert `value` from `from` to `to` using the built-in unit tables.
///
/// # Errors
/// Human-readable message when the value is not a number or the conversion
/// is unknown/incompatible.
pub fn convert_units(value_text: &str, from: &str, to: &str) -> Result<f64, String> {
    let v: f64 = value_text
        .trim()
        .parse()
        .map_err(|_| format!("{value_text:?} is not a number"))?;
    ocalc_units::builtin_converter()
        .convert(v, from, to)
        .map_err(|e| e.to_string())
}

/// All known unit symbols, sorted (for pick-lists).
#[must_use]
pub fn converter_symbols() -> Vec<String> {
    let mut syms = ocalc_units::builtin_converter().symbols();
    syms.sort();
    syms
}

/// Sample `expr` over `[from, to]` in `n` points via the engine.
///
/// # Errors
/// Human-readable message (range/parse/limit failures).
pub fn sample_graph_points(
    engine: &CalculatorEngine,
    expr: &str,
    from: f64,
    to: f64,
    n: usize,
) -> Result<Vec<GraphPoint>, String> {
    engine
        .sample_graph(expr, "x", from, to, n)
        .map_err(|e| e.to_string())
}

/// Parse a graph bound text box.
pub fn parse_bound(text: &str, fallback: f64) -> f64 {
    text.trim().parse::<f64>().unwrap_or(fallback)
}

/// Integer value in four bases for programmer mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseFormats {
    /// Decimal (exact).
    pub dec: String,
    /// Hexadecimal (`0x…`) or `—` when not integer-valued.
    pub hex: String,
    /// Octal (`0o…`) or `—`.
    pub oct: String,
    /// Binary (`0b…`) or `—`.
    pub bin: String,
    /// Whether the value was integer-valued.
    pub is_integer: bool,
}

/// Format `value` in dec/hex/oct/bin.
///
/// Integer-valued floats/decimals included (`3.0` → `3`). Non-integers and
/// complex values yield `—` placeholders for non-decimal bases.
pub fn format_bases(value: &Value) -> BaseFormats {
    let dec = value.to_string();
    let bigint = value.as_bigint().ok();
    match bigint {
        Some(b) => {
            use num_traits::Signed;
            let neg = b.sign() == num_bigint::Sign::Minus;
            let mag = b.abs();
            let prefix = if neg { "-" } else { "" };
            BaseFormats {
                dec,
                hex: format!("{prefix}0x{}", mag.to_str_radix(16).to_uppercase()),
                oct: format!("{prefix}0o{}", mag.to_str_radix(8)),
                bin: format!("{prefix}0b{}", mag.to_str_radix(2)),
                is_integer: true,
            }
        }
        None => BaseFormats {
            dec,
            hex: "—".to_owned(),
            oct: "—".to_owned(),
            bin: "—".to_owned(),
            is_integer: false,
        },
    }
}

// ---------------------------------------------------------------------------
// Gradians (UI-boundary transform, engine still does all math)
// ---------------------------------------------------------------------------

/// Direct trig functions whose *argument* is an angle.
const GRAD_DIRECT: [&str; 3] = ["sin", "cos", "tan"];
/// Inverse trig functions whose *result* is an angle.
const GRAD_INVERSE: [&str; 3] = ["asin", "acos", "atan"];

/// Rewrite `src` so gradian angles evaluate correctly with the engine in
/// radians: `sin(E)` → `sin((E)*pi/200)`, `asin(E)` → `((asin(E))*200/pi)`.
///
/// This is pure text handling (balanced-paren matching + case-insensitive
/// name lookup); every number still flows through the engine, which owns
/// `pi` and all trig. Non-trig calls, multi-arg calls like `atan2`, and
/// unknown names pass through untouched.
#[must_use]
pub fn grad_to_radian_expr(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len() + 16);
    transform_calls(src, bytes, 0, src.len(), &mut out);
    out
}

/// Recursively copy `src[start..end]` to `out`, rewriting trig calls.
fn transform_calls(src: &str, bytes: &[u8], mut i: usize, end: usize, out: &mut String) {
    while i < end {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let name_start = i;
            while i < end && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let name = &src[name_start..i];
            let lower = name.to_ascii_lowercase();
            // Skip whitespace between name and paren.
            let mut j = i;
            while j < end && (bytes[j] as char).is_ascii_whitespace() {
                j += 1;
            }
            if j < end && bytes[j] == b'(' {
                if let Some(close) = match_paren(bytes, j, end) {
                    let inner = &src[j + 1..close];
                    if GRAD_DIRECT.contains(&lower.as_str()) {
                        // sin(E) → sin((E)*pi/200), E transformed recursively.
                        let mut inner_out = String::new();
                        transform_calls(inner, inner.as_bytes(), 0, inner.len(), &mut inner_out);
                        out.push_str(name);
                        out.push('(');
                        out.push('(');
                        out.push_str(&inner_out);
                        out.push_str(")*pi/200)");
                        i = close + 1;
                        continue;
                    }
                    if GRAD_INVERSE.contains(&lower.as_str()) {
                        // asin(E) → ((asin(E))*200/pi).
                        let mut inner_out = String::new();
                        transform_calls(inner, inner.as_bytes(), 0, inner.len(), &mut inner_out);
                        out.push_str("((");
                        out.push_str(name);
                        out.push('(');
                        out.push_str(&inner_out);
                        out.push_str("))*200/pi)");
                        i = close + 1;
                        continue;
                    }
                    // Other call: keep the call, transform the arguments.
                    out.push_str(name);
                    out.push_str(&src[i..j]);
                    out.push('(');
                    transform_calls(src, bytes, j + 1, close, out);
                    out.push(')');
                    i = close + 1;
                    continue;
                }
            }
            out.push_str(name);
        } else if c == '(' {
            if let Some(close) = match_paren(bytes, i, end) {
                out.push('(');
                transform_calls(src, bytes, i + 1, close, out);
                out.push(')');
                i = close + 1;
            } else {
                out.push(c);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
}

/// Byte index of the `)` matching the `(` at `open`, within `end`.
fn match_paren(bytes: &[u8], open: usize, end: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < end {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Speed conversion (composed from public engine APIs only)
// ---------------------------------------------------------------------------

/// Length symbols usable as a speed numerator (engine Length category).
#[must_use]
pub fn speed_length_symbols() -> Vec<String> {
    symbols_in_category("length")
}

/// Time symbols usable as a speed denominator (engine Time category).
#[must_use]
pub fn speed_time_symbols() -> Vec<String> {
    symbols_in_category("time")
}

fn symbols_in_category(category: &str) -> Vec<String> {
    let conv = ocalc_units::builtin_converter();
    let mut syms: Vec<String> = converter_symbols()
        .into_iter()
        .filter(|s| {
            conv.lookup(s)
                .is_some_and(|d| d.category.to_string() == category)
        })
        .collect();
    syms.sort();
    syms
}

/// Canonical symbols of one engine category (for category pickers).
#[must_use]
pub fn symbols_in_category_public(category: &str) -> Vec<String> {
    symbols_in_category(category)
}

/// Convert `value` from `from_len/from_time` to `to_len/to_time`.
///
/// Factors come from the public unit converter; the arithmetic itself runs
/// through `engine.eval` so no conversion math lives in the UI.
///
/// # Errors
/// Human-readable message for bad input, unknown units, or engine failures.
pub fn convert_speed(
    engine: &mut CalculatorEngine,
    value_text: &str,
    from_len: &str,
    from_time: &str,
    to_len: &str,
    to_time: &str,
) -> Result<f64, String> {
    let v: f64 = value_text
        .trim()
        .parse()
        .map_err(|_| format!("{value_text:?} is not a number"))?;
    let conv = ocalc_units::builtin_converter();
    // Factors to base units (m, s) via the public converter.
    let lf = conv
        .convert(1.0, from_len, "m")
        .map_err(|e| e.to_string())?;
    let tf = conv
        .convert(1.0, from_time, "s")
        .map_err(|e| e.to_string())?;
    let lt = conv.convert(1.0, to_len, "m").map_err(|e| e.to_string())?;
    let tt = conv.convert(1.0, to_time, "s").map_err(|e| e.to_string())?;
    // value * (lf/tf) / (lt/tt), evaluated by the engine.
    let expr = format!("({v}) * ({lf}) / ({tf}) * ({tt}) / ({lt})");
    engine
        .eval(&expr)
        .map_err(|e| e.to_string())
        .map(|n| n.to_f64())
}

// ---------------------------------------------------------------------------
// Memory arithmetic (public Value API only)
// ---------------------------------------------------------------------------

/// Add `other` to memory via the public [`Value`] arithmetic.
pub fn memory_add(mem: &Value, other: &Value) -> Result<Value, String> {
    mem.add(other).map_err(|e| e.to_string())
}

/// Subtract `other` from memory via the public [`Value`] arithmetic.
pub fn memory_sub(mem: &Value, other: &Value) -> Result<Value, String> {
    mem.sub(other).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// History maintenance using only the public history API
// ---------------------------------------------------------------------------

/// Rebuild `store`, keeping entries that satisfy `keep` and at most
/// `max_len` newest entries, using only `to_json`/`from_json`.
///
/// Ids and `next_id` survive (the JSON payload carries them), so sidebar
/// selection stays valid. Used for per-item delete and history-size caps.
pub fn rebuild_history(
    store: &ocalc_history::HistoryStore,
    keep: &HashSet<u64>,
    max_len: Option<usize>,
) -> Result<ocalc_history::HistoryStore, String> {
    let json = store.to_json().map_err(|e| e.to_string())?;
    let mut v: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("history encode: {e}"))?;
    let entries = v
        .get_mut("entries")
        .and_then(|e| e.as_array_mut())
        .ok_or_else(|| "history payload has no entries".to_owned())?;
    entries.retain(|e| {
        e.get("id")
            .and_then(|id| id.as_u64())
            .is_some_and(|id| keep.contains(&id))
    });
    if let Some(max) = max_len {
        if entries.len() > max {
            let drop = entries.len() - max;
            entries.drain(..drop);
        }
    }
    let back = serde_json::to_string(&v).map_err(|e| format!("history encode: {e}"))?;
    ocalc_history::HistoryStore::from_json(&back).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// CAS frontend (symbolic, via `ocalc-cas` public API only)
//
// The engine evaluates to numbers and cannot carry symbolic trees, so this
// section parses UI text into [`SymExpr`](ocalc_cas::SymExpr) with a small
// frontend parser (numbers, `x`/`y`/`z` variables, `+ - * / ^`, parens,
// implicit multiplication, `sin`/`cos`/`log`) and delegates every rewrite to
// the CAS crate (`simplify`, `expand`, `factor`, `differentiate`,
// `integrate`, `solve_linear`, `solve_quadratic`). No symbolic logic lives
// in the UI.
// ---------------------------------------------------------------------------

use ocalc_cas::SymExpr;

/// Parse UI text into a symbolic expression.
///
/// Accepts numbers, variables (`x`, `y`, `z`, …), `+ - * / ^` (`**` also
/// works), parentheses, unary minus, implicit multiplication (`2x`,
/// `2(x+1)`), and `sin(…)` / `cos(…)` / `log(…)`.
///
/// # Errors
/// Human-readable message with the offending position.
pub fn cas_parse(src: &str) -> Result<SymExpr, String> {
    let tokens = cas_tokenize(src)?;
    let mut parser = CasParser {
        tokens: &tokens,
        pos: 0,
    };
    let expr = parser.parse_expr()?;
    match parser.peek() {
        CasTok::End => Ok(expr),
        other => Err(format!("unexpected {other} after expression")),
    }
}

/// Validate a differentiation variable (`x`, `y`, …) via the CAS crate.
///
/// Returns the trimmed name.
///
/// # Errors
/// Empty or non-identifier names (message comes from the CAS crate).
pub fn cas_valid_var(var: &str) -> Result<String, String> {
    let name = var.trim();
    if name.is_empty() {
        return Err("variable must not be empty (try x)".to_owned());
    }
    ocalc_cas::Symbol::new(name).map(|s| s.name().to_owned())
}

/// Simplify `src` (`x + x → 2*x`).
///
/// # Errors
/// Parse failures.
pub fn cas_simplify(src: &str) -> Result<String, String> {
    Ok(ocalc_cas::simplify(&cas_parse(src)?).to_string())
}

/// Expand `src` (`(x+1)^2 → x^2 + 2*x + 1`).
///
/// # Errors
/// Parse failures.
pub fn cas_expand(src: &str) -> Result<String, String> {
    Ok(ocalc_cas::expand(&cas_parse(src)?).to_string())
}

/// Factor `src` (verified by re-expansion; input returned when prime).
///
/// # Errors
/// Parse failures.
pub fn cas_factor(src: &str) -> Result<String, String> {
    Ok(ocalc_cas::factor(&cas_parse(src)?).to_string())
}

/// Differentiate `src` with respect to `var` (`d/dx x^2 = 2*x`).
///
/// # Errors
/// Parse failures, bad variable names, or variable exponents.
pub fn cas_differentiate(src: &str, var: &str) -> Result<String, String> {
    let name = cas_valid_var(var)?;
    Ok(ocalc_cas::differentiate(&cas_parse(src)?, &name)?.to_string())
}

/// Integrate `src` with respect to `var` (`integral x dx = 0.5*x^2`).
///
/// # Errors
/// Parse failures, bad variable names, or unsupported forms.
pub fn cas_integrate(src: &str, var: &str) -> Result<String, String> {
    let name = cas_valid_var(var)?;
    Ok(ocalc_cas::integrate(&cas_parse(src)?, &name)?.to_string())
}

/// Solve `a·x + b = 0`; returns a `x = …` string.
///
/// # Errors
/// Non-numeric coefficients or `a == 0`.
pub fn cas_solve_linear(a_text: &str, b_text: &str) -> Result<String, String> {
    let a = parse_named_number(a_text, "a")?;
    let b = parse_named_number(b_text, "b")?;
    Ok(format!(
        "x = {}",
        fmt_scalar(ocalc_cas::solve_linear(a, b)?)
    ))
}

/// Solve `a·x² + b·x + c = 0`; returns `x = …` / `x₁, x₂` / no-roots strings.
///
/// # Errors
/// Non-numeric coefficients or `a == 0`.
pub fn cas_solve_quadratic(a_text: &str, b_text: &str, c_text: &str) -> Result<String, String> {
    let a = parse_named_number(a_text, "a")?;
    let b = parse_named_number(b_text, "b")?;
    let c = parse_named_number(c_text, "c")?;
    let roots = ocalc_cas::solve_quadratic(a, b, c)?;
    match roots.as_slice() {
        [] => Ok("no real solutions".to_owned()),
        [r] => Ok(format!("x = {}", fmt_scalar(*r))),
        [r1, r2] => Ok(format!("x = {} or {}", fmt_scalar(*r1), fmt_scalar(*r2))),
        _ => Err("solver returned too many roots".to_owned()),
    }
}

/// Parse a labeled numeric field.
///
/// # Errors
/// Empty or non-numeric input.
fn parse_named_number(text: &str, label: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .map_err(|_| format!("{label} must be a number, got {text:?}"))
}

/// Format a scalar: integers print without a decimal point.
fn fmt_scalar(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Frontend CAS token.
#[derive(Debug, Clone, PartialEq)]
enum CasTok {
    /// Numeric literal.
    Num(f64),
    /// Identifier (variable or function name).
    Ident(String),
    /// Operators / punctuation.
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    /// Parentheses.
    LParen,
    RParen,
    /// End of input.
    End,
}

impl std::fmt::Display for CasTok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Num(v) => write!(f, "{v}"),
            Self::Ident(s) => write!(f, "{s}"),
            Self::Plus => write!(f, "'+'"),
            Self::Minus => write!(f, "'-'"),
            Self::Star => write!(f, "'*'"),
            Self::Slash => write!(f, "'/'"),
            Self::Caret => write!(f, "'^'"),
            Self::LParen => write!(f, "'('"),
            Self::RParen => write!(f, "')'"),
            Self::End => write!(f, "end of input"),
        }
    }
}

/// Tokenize CAS input (whitespace ignored).
fn cas_tokenize(src: &str) -> Result<Vec<CasTok>, String> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                let mut j = i + 1;
                if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                    j += 1;
                    while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text = &src[start..i];
            text.parse::<f64>()
                .map(CasTok::Num)
                .map_err(|_| format!("bad number {text:?} at position {start}"))
                .map(|t| out.push(t))?;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len()
                && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(CasTok::Ident(src[start..i].to_owned()));
        } else {
            match c {
                '+' => {
                    out.push(CasTok::Plus);
                    i += 1;
                }
                '-' => {
                    out.push(CasTok::Minus);
                    i += 1;
                }
                '*' => {
                    // `**` is power, like the engine.
                    if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        out.push(CasTok::Caret);
                        i += 2;
                    } else {
                        out.push(CasTok::Star);
                        i += 1;
                    }
                }
                '/' => {
                    out.push(CasTok::Slash);
                    i += 1;
                }
                '^' => {
                    out.push(CasTok::Caret);
                    i += 1;
                }
                '(' => {
                    out.push(CasTok::LParen);
                    i += 1;
                }
                ')' => {
                    out.push(CasTok::RParen);
                    i += 1;
                }
                _ => return Err(format!("unexpected {c:?} at position {i}")),
            }
        }
    }
    out.push(CasTok::End);
    Ok(out)
}

/// Recursive-descent parser over [`CasTok`]: `+ -` → `* /` (+ implicit
/// multiplication) → `^` (right-assoc) → unary → primary.
struct CasParser<'a> {
    /// Token stream (ends with [`CasTok::End`]).
    tokens: &'a [CasTok],
    /// Current position.
    pos: usize,
}

impl CasParser<'_> {
    /// Peek at the current token.
    fn peek(&self) -> CasTok {
        self.tokens.get(self.pos).cloned().unwrap_or(CasTok::End)
    }

    /// Parse `a ± b ± …`.
    fn parse_expr(&mut self) -> Result<SymExpr, String> {
        let mut terms = vec![self.parse_term()?];
        // Track signs separately: `a - b` is `a + (-b)`.
        let mut negated = vec![false];
        loop {
            match self.peek() {
                CasTok::Plus => {
                    self.pos += 1;
                    terms.push(self.parse_term()?);
                    negated.push(false);
                }
                CasTok::Minus => {
                    self.pos += 1;
                    terms.push(self.parse_term()?);
                    negated.push(true);
                }
                _ => break,
            }
        }
        let mut out = Vec::with_capacity(terms.len());
        for (t, neg) in terms.into_iter().zip(negated) {
            if neg {
                out.push(SymExpr::Neg(Box::new(t)));
            } else {
                out.push(t);
            }
        }
        Ok(if out.len() == 1 {
            out.pop().unwrap_or(SymExpr::Const(0.0))
        } else {
            SymExpr::Add(out)
        })
    }

    /// Parse `a * b / c`, `2x`, `2(x+1)`.
    fn parse_term(&mut self) -> Result<SymExpr, String> {
        let mut factors = vec![self.parse_factor()?];
        loop {
            match self.peek() {
                CasTok::Star => {
                    self.pos += 1;
                    factors.push(self.parse_factor()?);
                }
                CasTok::Slash => {
                    self.pos += 1;
                    let denom = self.parse_factor()?;
                    factors.push(SymExpr::Pow(
                        Box::new(denom),
                        Box::new(SymExpr::Const(-1.0)),
                    ));
                }
                CasTok::Num(_) | CasTok::Ident(_) | CasTok::LParen => {
                    factors.push(self.parse_factor()?);
                }
                _ => break,
            }
        }
        Ok(if factors.len() == 1 {
            factors.pop().unwrap_or(SymExpr::Const(1.0))
        } else {
            SymExpr::Mul(factors)
        })
    }

    /// Parse right-associative power.
    fn parse_factor(&mut self) -> Result<SymExpr, String> {
        let base = self.parse_unary()?;
        if self.peek() == CasTok::Caret {
            self.pos += 1;
            let exp = self.parse_factor()?;
            Ok(SymExpr::Pow(Box::new(base), Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    /// Parse unary minus (plus is a no-op).
    fn parse_unary(&mut self) -> Result<SymExpr, String> {
        match self.peek() {
            CasTok::Minus => {
                self.pos += 1;
                Ok(SymExpr::Neg(Box::new(self.parse_unary()?)))
            }
            CasTok::Plus => {
                self.pos += 1;
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    /// Parse numbers, variables, calls, and parenthesized groups.
    fn parse_primary(&mut self) -> Result<SymExpr, String> {
        match self.peek() {
            CasTok::Num(v) => {
                self.pos += 1;
                Ok(SymExpr::Const(v))
            }
            CasTok::Ident(name) => {
                self.pos += 1;
                if self.peek() == CasTok::LParen {
                    self.pos += 1;
                    let arg = self.parse_expr()?;
                    if self.peek() != CasTok::RParen {
                        return Err(format!("expected ')' after {name}(…)"));
                    }
                    self.pos += 1;
                    match name.to_ascii_lowercase().as_str() {
                        "sin" => Ok(ocalc_cas::sin(arg)),
                        "cos" => Ok(ocalc_cas::cos(arg)),
                        "log" => Ok(ocalc_cas::log(arg)),
                        _ => Err(format!("unknown function {name:?} (try sin, cos, log)")),
                    }
                } else {
                    Ok(SymExpr::Var(name))
                }
            }
            CasTok::LParen => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                if self.peek() != CasTok::RParen {
                    return Err("expected ')'".to_owned());
                }
                self.pos += 1;
                Ok(inner)
            }
            other => Err(format!("unexpected {other} in expression")),
        }
    }
}

// ---------------------------------------------------------------------------
// Physics frontend (via `ocalc-physics` public API only)
// ---------------------------------------------------------------------------

/// One physics constant for the reference panel: name, value, unit.
#[derive(Debug, Clone)]
pub struct PhysicsConstant {
    /// Display name, e.g. `"gravity"`.
    pub name: &'static str,
    /// Value in SI units.
    pub value: f64,
    /// Unit label, e.g. `"m/s²"`.
    pub unit: &'static str,
}

/// Reference constants (read-only values from the physics crate).
#[must_use]
pub fn physics_constants() -> Vec<PhysicsConstant> {
    vec![
        PhysicsConstant {
            name: "gravity",
            value: ocalc_physics::GRAVITY,
            unit: "m/s²",
        },
        PhysicsConstant {
            name: "speed of light",
            value: ocalc_physics::SPEED_OF_LIGHT,
            unit: "m/s",
        },
        PhysicsConstant {
            name: "Planck",
            value: ocalc_physics::PLANCK,
            unit: "J·s",
        },
        PhysicsConstant {
            name: "elementary charge",
            value: ocalc_physics::ELEMENTARY_CHARGE,
            unit: "C",
        },
        PhysicsConstant {
            name: "Avogadro",
            value: ocalc_physics::AVOGADRO,
            unit: "mol⁻¹",
        },
        PhysicsConstant {
            name: "Boltzmann",
            value: ocalc_physics::BOLTZMANN,
            unit: "J/K",
        },
    ]
}

/// Compute physics formula `id` from two text fields.
///
/// `b_text` is ignored for single-input formulas (`weight`).
/// Returns the formatted value plus its SI unit.
///
/// # Errors
/// Unknown ids, non-numeric inputs, or domain errors from the physics crate
/// (zero time, zero resistance, …).
pub fn physics_compute(
    id: &str,
    a_text: &str,
    b_text: &str,
    precision: u8,
) -> Result<(String, &'static str), String> {
    let value: f64 = match id {
        "force" => {
            let (m, a) = (
                parse_named_number(a_text, "mass"),
                parse_named_number(b_text, "acceleration")?,
            );
            ocalc_physics::force(m?, a)
        }
        "weight" => ocalc_physics::weight(parse_named_number(a_text, "mass")?),
        "velocity" => ocalc_physics::velocity(
            parse_named_number(a_text, "distance")?,
            parse_named_number(b_text, "time")?,
        )?,
        "acceleration" => ocalc_physics::acceleration(
            parse_named_number(a_text, "Δv")?,
            parse_named_number(b_text, "time")?,
        )?,
        "momentum" => ocalc_physics::momentum(
            parse_named_number(a_text, "mass")?,
            parse_named_number(b_text, "velocity")?,
        ),
        "kinetic_energy" => ocalc_physics::kinetic_energy(
            parse_named_number(a_text, "mass")?,
            parse_named_number(b_text, "velocity")?,
        )?,
        "potential_energy" => ocalc_physics::potential_energy(
            parse_named_number(a_text, "mass")?,
            parse_named_number(b_text, "height")?,
        ),
        "voltage" => ocalc_physics::voltage(
            parse_named_number(a_text, "current")?,
            parse_named_number(b_text, "resistance")?,
        ),
        "current" => ocalc_physics::current(
            parse_named_number(a_text, "voltage")?,
            parse_named_number(b_text, "resistance")?,
        )?,
        "resistance" => ocalc_physics::resistance(
            parse_named_number(a_text, "voltage")?,
            parse_named_number(b_text, "current")?,
        )?,
        "power_vi" => ocalc_physics::power_vi(
            parse_named_number(a_text, "voltage")?,
            parse_named_number(b_text, "current")?,
        ),
        "power_ri" => ocalc_physics::power_ri(
            parse_named_number(a_text, "current")?,
            parse_named_number(b_text, "resistance")?,
        ),
        "power_vr" => ocalc_physics::power_vr(
            parse_named_number(a_text, "voltage")?,
            parse_named_number(b_text, "resistance")?,
        )?,
        _ => return Err(format!("unknown physics formula {id:?}")),
    };
    let unit: &'static str = match id {
        "force" | "weight" => "N",
        "velocity" => "m/s",
        "acceleration" => "m/s²",
        "momentum" => "kg·m/s",
        "kinetic_energy" | "potential_energy" => "J",
        "voltage" => "V",
        "current" => "A",
        "resistance" => "Ω",
        "power_vi" | "power_ri" | "power_vr" => "W",
        _ => "",
    };
    Ok((format_value(&Value::Float(value), precision), unit))
}

// ---------------------------------------------------------------------------
// Chemistry frontend (via `ocalc-chemistry` public API only)
// ---------------------------------------------------------------------------

/// One element's contribution to a compound (display copy).
#[derive(Debug, Clone)]
pub struct ChemistryComponent {
    /// Symbol, e.g. `"H"`.
    pub symbol: String,
    /// Name, e.g. `"Hydrogen"`.
    pub name: String,
    /// Atomic number.
    pub number: u32,
    /// Atom count in the formula.
    pub count: u32,
    /// `count × atomic weight` (g/mol).
    pub mass: f64,
    /// Mass fraction in percent.
    pub percent: f64,
}

/// Formula analysis result: total molar mass + per-element breakdown.
#[derive(Debug, Clone)]
pub struct ChemistryReport {
    /// Total molar mass (g/mol).
    pub mass: f64,
    /// Per-element breakdown.
    pub components: Vec<ChemistryComponent>,
}

/// Analyze `formula` (`H2O`, `Ca(OH)2`, …).
///
/// # Errors
/// Whatever the chemistry crate's parser rejects.
pub fn chemistry_analyze(formula: &str) -> Result<ChemistryReport, String> {
    let formula = formula.trim();
    if formula.is_empty() {
        return Err("enter a formula, e.g. H2O".to_owned());
    }
    let parts = ocalc_chemistry::composition(formula)?;
    let mass = parts.iter().map(|c| c.mass).sum();
    Ok(ChemistryReport {
        mass,
        components: parts
            .into_iter()
            .map(|c| ChemistryComponent {
                symbol: c.element.symbol.to_owned(),
                name: c.element.name.to_owned(),
                number: c.element.number,
                count: c.count,
                mass: c.mass,
                percent: c.percent,
            })
            .collect(),
    })
}

/// One periodic-table entry for the browser (display copy).
#[derive(Debug, Clone)]
pub struct ElementCard {
    /// Symbol, e.g. `"He"`.
    pub symbol: String,
    /// Name, e.g. `"Helium"`.
    pub name: String,
    /// Atomic number.
    pub number: u32,
    /// Standard atomic weight (g/mol).
    pub mass: f64,
}

/// Look up one element by symbol or name (`He`, `oxygen`).
#[must_use]
pub fn chemistry_element(query: &str) -> Option<ElementCard> {
    let e = ocalc_chemistry::table::lookup(query.trim())?;
    Some(ElementCard {
        symbol: e.symbol.to_owned(),
        name: e.name.to_owned(),
        number: e.number,
        mass: e.mass,
    })
}

/// All 118 elements in atomic-number order.
#[must_use]
pub fn chemistry_table() -> Vec<ElementCard> {
    ocalc_chemistry::table::TABLE
        .iter()
        .map(|e| ElementCard {
            symbol: e.symbol.to_owned(),
            name: e.name.to_owned(),
            number: e.number,
            mass: e.mass,
        })
        .collect()
}

/// Filter the table by symbol / name / number substring (case-insensitive).
/// Empty query returns the full table.
#[must_use]
pub fn filter_elements(query: &str) -> Vec<ElementCard> {
    let q = query.trim().to_ascii_lowercase();
    let all = chemistry_table();
    if q.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|e| {
            e.symbol.to_ascii_lowercase().contains(&q)
                || e.name.to_ascii_lowercase().contains(&q)
                || e.number.to_string().contains(&q)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Finance frontend (via `ocalc-finance` public API only)
//
// Rates are entered as human percentages (`5` = 5 %) and converted to the
// decimal fractions the finance crate expects.
// ---------------------------------------------------------------------------

/// Compound future value from principal, annual rate (%), periods.
///
/// # Errors
/// Non-numeric or out-of-range inputs.
pub fn finance_compound(p_text: &str, rate_pct_text: &str, n_text: &str) -> Result<f64, String> {
    ocalc_finance::compound(
        parse_named_number(p_text, "principal")?,
        parse_named_number(rate_pct_text, "rate")? / 100.0,
        parse_named_number(n_text, "periods")?,
    )
}

/// Effective annual rate (fraction) from nominal rate (%) and frequency.
///
/// # Errors
/// Non-numeric or out-of-range inputs.
pub fn finance_effective_rate(nominal_pct_text: &str, m_text: &str) -> Result<f64, String> {
    ocalc_finance::effective_rate(
        parse_named_number(nominal_pct_text, "nominal rate")? / 100.0,
        parse_named_number(m_text, "compounds/year")?,
    )
}

/// Loan amortization: monthly payment, lifetime total, lifetime interest.
///
/// # Errors
/// Non-numeric or out-of-range inputs.
pub fn finance_loan(
    p_text: &str,
    apr_pct_text: &str,
    months_text: &str,
) -> Result<(f64, f64, f64), String> {
    let (p, r, n) = (
        parse_named_number(p_text, "principal")?,
        parse_named_number(apr_pct_text, "APR")? / 100.0,
        parse_named_number(months_text, "months")?,
    );
    Ok((
        ocalc_finance::loan_payment(p, r, n)?,
        ocalc_finance::loan_total(p, r, n)?,
        ocalc_finance::loan_interest(p, r, n)?,
    ))
}

/// Compounded growth value from start, rate (%), periods.
///
/// # Errors
/// Non-numeric or out-of-range inputs.
pub fn finance_growth(start_text: &str, rate_pct_text: &str, n_text: &str) -> Result<f64, String> {
    ocalc_finance::growth(
        parse_named_number(start_text, "start")?,
        parse_named_number(rate_pct_text, "rate")? / 100.0,
        parse_named_number(n_text, "periods")?,
    )
}

/// Per-period growth rate (fraction) from start → end in `periods` steps.
///
/// # Errors
/// Non-numeric or out-of-range inputs.
pub fn finance_growth_rate(start_text: &str, end_text: &str, n_text: &str) -> Result<f64, String> {
    ocalc_finance::growth_rate(
        parse_named_number(start_text, "start")?,
        parse_named_number(end_text, "end")?,
        parse_named_number(n_text, "periods")?,
    )
}

/// `pct` percent of `base`.
///
/// # Errors
/// Non-numeric inputs.
pub fn finance_percent(base_text: &str, pct_text: &str) -> Result<f64, String> {
    ocalc_finance::percent(
        parse_named_number(base_text, "base")?,
        parse_named_number(pct_text, "percent")?,
    )
}

/// Percentage change from `old` to `new` (percentage points).
///
/// # Errors
/// Non-numeric inputs or `old == 0`.
pub fn finance_percent_change(old_text: &str, new_text: &str) -> Result<f64, String> {
    ocalc_finance::percent_change(
        parse_named_number(old_text, "old value")?,
        parse_named_number(new_text, "new value")?,
    )
}

// ---------------------------------------------------------------------------
// Settings persistence (XDG JSON file, best-effort)
// ---------------------------------------------------------------------------

/// Default settings file: `<XDG config>/ocalculator/settings.json`
/// (e.g. `~/.config/ocalculator/settings.json`).
///
/// Legacy `~/.config/ocalc-desktop/settings.json` is migrated on startup
/// (see [`ocalc_history::migrate_legacy`]); this function always returns the
/// new location.
#[must_use]
pub fn default_settings_path() -> std::path::PathBuf {
    ocalc_history::settings_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> CalculatorEngine {
        CalculatorEngine::new()
    }

    #[test]
    fn eval_ok_and_err() {
        let mut e = engine();
        let ok = evaluate(&mut e, "1 + 2 * 3", 6);
        assert_eq!(ok.display, "7");
        assert!(ok.error.is_none());
        let err = evaluate(&mut e, "1 + ", 6);
        assert!(err.result.is_none());
        assert!(err.error.is_some());
        let empty = evaluate(&mut e, "   ", 6);
        assert!(empty.display.is_empty() && empty.error.is_none());
    }

    #[test]
    fn precision_rounds_floats_only() {
        assert_eq!(format_value(&Value::Float(2.5), 0), "2");
        assert_eq!(format_value(&Value::Float(2.567), 2), "2.57");
        assert_eq!(format_value(&Value::Integer(42), 0), "42");
        assert_eq!(format_value(&Value::complex(1.0, 2.0), 0), "1 + 2i");
    }

    #[test]
    fn scientific_notation_formats() {
        // Standard mode delegates to the classic formatter.
        assert_eq!(format_number(&Value::Float(2.567), 2, false), "2.57");
        // Scientific mode: trimmed mantissa plus exponent.
        assert_eq!(format_number(&Value::Float(15000.0), 6, true), "1.5e4");
        assert_eq!(format_number(&Value::Float(0.00123), 6, true), "1.23e-3");
        assert_eq!(format_number(&Value::Float(2.5), 0, true), "2.5e0");
        assert_eq!(format_number(&Value::Float(0.0), 6, true), "0");
        // Exact variants are never touched, even in scientific mode.
        assert_eq!(format_number(&Value::Integer(42), 6, true), "42");
        assert_eq!(format_number(&Value::complex(1.0, 2.0), 6, true), "1 + 2i");
        // Non-finite floats keep their display names.
        assert_eq!(format_number(&Value::Float(f64::INFINITY), 6, true), "inf");
        // Precision caps the mantissa digits.
        assert_eq!(format_number(&Value::Float(1234.5678), 2, true), "1.23e3");
    }

    #[test]
    fn convert_known_units() {
        let v = convert_units("1", "km", "m").unwrap();
        assert!((v - 1000.0).abs() < 1e-9);
        assert!(convert_units("abc", "km", "m").is_err());
        assert!(convert_units("1", "km", "nosuchunit").is_err());
    }

    #[test]
    fn symbols_nonempty_sorted() {
        let s = converter_symbols();
        assert!(!s.is_empty());
        let mut sorted = s.clone();
        sorted.sort();
        assert_eq!(s, sorted);
    }

    #[test]
    fn graph_ok_and_err() {
        let e = engine();
        let pts = sample_graph_points(&e, "x", -1.0, 1.0, 3).unwrap();
        assert_eq!(pts.len(), 3);
        assert!(sample_graph_points(&e, "???", 0.0, 1.0, 3).is_err());
    }

    #[test]
    fn bases_for_integers_and_floats() {
        let f = format_bases(&Value::Integer(255));
        assert_eq!(f.hex, "0xFF");
        assert_eq!(f.oct, "0o377");
        assert_eq!(f.bin, "0b11111111");
        let nf = format_bases(&Value::Float(1.5));
        assert!(!nf.is_integer && nf.hex == "—");
    }

    #[test]
    fn grad_transform_direct_and_inverse() {
        assert_eq!(grad_to_radian_expr("sin(100)"), "sin((100)*pi/200)");
        assert_eq!(grad_to_radian_expr("SIN(100)"), "SIN((100)*pi/200)");
        assert_eq!(grad_to_radian_expr("asin(1)"), "((asin(1))*200/pi)");
        // Nested: inner first, both scaled exactly once.
        assert_eq!(
            grad_to_radian_expr("sin(sin(100))"),
            "sin((sin((100)*pi/200))*pi/200)"
        );
        // Mixed expression: only trig calls rewritten.
        assert_eq!(
            grad_to_radian_expr("sin(100) + x * cos(y)"),
            "sin((100)*pi/200) + x * cos((y)*pi/200)"
        );
        // Untouched: multi-arg, hyperbolic, unknown names.
        assert_eq!(grad_to_radian_expr("atan2(1, 2)"), "atan2(1, 2)");
        assert_eq!(grad_to_radian_expr("sinh(1)"), "sinh(1)");
        assert_eq!(grad_to_radian_expr("sine(1)"), "sine(1)");
        assert_eq!(grad_to_radian_expr("1 + 2"), "1 + 2");
    }

    #[test]
    fn grad_evaluates_correctly() {
        // 100 grad = 90° → sin = 1; 200 grad = 180° → cos = -1.
        let mut e = engine();
        for (src, want) in [("sin(100)", 1.0), ("cos(200)", -1.0)] {
            let rewritten = grad_to_radian_expr(src);
            let v = e.eval(&rewritten).unwrap().to_f64();
            assert!((v - want).abs() < 1e-9, "{src} → {rewritten} = {v}");
        }
        let v = e.eval(&grad_to_radian_expr("asin(1)")).unwrap().to_f64();
        assert!((v - 100.0).abs() < 1e-9);
    }

    #[test]
    fn speed_conversion() {
        let mut e = engine();
        // 100 km/h ≈ 27.777… m/s; 60 mph ≈ 26.8224 m/s.
        let v = convert_speed(&mut e, "100", "km", "h", "m", "s").unwrap();
        assert!((v - 27.777_777_777_777_78).abs() < 1e-9);
        let v = convert_speed(&mut e, "60", "mi", "h", "m", "s").unwrap();
        assert!((v - 26.8224).abs() < 1e-6);
        assert!(convert_speed(&mut e, "abc", "km", "h", "m", "s").is_err());
        assert!(convert_speed(&mut e, "1", "km", "h", "m", "nosuch").is_err());
        assert!(!speed_length_symbols().is_empty());
        assert!(!speed_time_symbols().is_empty());
    }

    #[test]
    fn memory_add_sub() {
        let m = Value::Integer(10);
        assert_eq!(
            memory_add(&m, &Value::Integer(5)).unwrap().to_string(),
            "15"
        );
        assert_eq!(memory_sub(&m, &Value::Integer(4)).unwrap().to_string(), "6");
    }

    #[test]
    fn history_rebuild_keeps_ids() {
        use ocalc_history::HistoryStore;
        let mut h = HistoryStore::new();
        h.push("1+1", "2");
        h.push("2+2", "4");
        h.push("3+3", "6");
        let drop_first: HashSet<u64> = h.all().iter().skip(1).map(|e| e.id).collect();
        let back = rebuild_history(&h, &drop_first, None).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.all()[0].expression, "2+2");
        // next_id monotonic: new pushes continue past the max id.
        let mut back = back;
        let e = back.push("4+4", "8");
        assert_eq!(e.id, 4);
        // Capacity cap keeps newest.
        let keep: HashSet<u64> = h.all().iter().map(|e| e.id).collect();
        let capped = rebuild_history(&h, &keep, Some(2)).unwrap();
        assert_eq!(capped.len(), 2);
        assert_eq!(capped.all()[0].expression, "2+2");
    }

    #[test]
    fn cas_parser_shapes() {
        assert_eq!(cas_parse("x").unwrap().to_string(), "x");
        assert_eq!(cas_parse(" 2 ").unwrap().to_string(), "2");
        assert_eq!(cas_parse("x + x").unwrap().to_string(), "x + x");
        // Precedence: `*` binds tighter; `^` is right-associative.
        assert_eq!(cas_parse("1 + 2 * 3").unwrap().to_string(), "1 + 2*3");
        assert_eq!(cas_parse("2^3^2").unwrap().to_string(), "2^3^2");
        // Unary minus and implicit multiplication.
        assert_eq!(cas_parse("-x").unwrap().to_string(), "-x");
        assert_eq!(cas_parse("2x").unwrap().to_string(), "2*x");
        assert_eq!(cas_parse("2(x + 1)").unwrap().to_string(), "2*(x + 1)");
        // `**` matches the engine; calls are case-insensitive.
        assert_eq!(cas_parse("2**3").unwrap().to_string(), "2^3");
        assert_eq!(cas_parse("SIN(x)").unwrap().to_string(), "sin(x)");
        assert_eq!(
            cas_parse("cos(x) + log(x)").unwrap().to_string(),
            "cos(x) + log(x)"
        );
        // Division becomes a negative power (no Div node in SymExpr).
        assert_eq!(cas_parse("x / y").unwrap().to_string(), "x*y^-1");
    }

    #[test]
    fn cas_parser_rejects_bad_input() {
        assert!(cas_parse("").is_err());
        assert!(cas_parse("   ").is_err());
        assert!(cas_parse("1 + ").is_err());
        assert!(cas_parse("(x").is_err());
        assert!(cas_parse("x)").is_err());
        assert!(cas_parse("foo(x)").is_err());
        assert!(cas_parse("1 & 2").is_err());
        assert!(cas_parse("x + y z)").is_err());
        assert!(cas_valid_var("").is_err());
        assert!(cas_valid_var("1x").is_err());
        assert_eq!(cas_valid_var(" x ").unwrap(), "x");
    }

    #[test]
    fn cas_operations_end_to_end() {
        assert_eq!(cas_simplify("x + x").unwrap(), "2*x");
        assert_eq!(cas_simplify("x * x").unwrap(), "x^2");
        assert_eq!(cas_expand("(x + 1)^2").unwrap(), "x^2 + 2*x + 1");
        assert_eq!(cas_factor("2*x + 4").unwrap(), "2*(x + 2)");
        assert_eq!(cas_differentiate("x^2", "x").unwrap(), "2*x");
        assert_eq!(cas_integrate("x", "x").unwrap(), "0.5*x^2");
        assert_eq!(cas_solve_linear("2", "-4").unwrap(), "x = 2");
        assert_eq!(cas_solve_quadratic("1", "0", "-4").unwrap(), "x = -2 or 2");
        assert_eq!(cas_solve_quadratic("1", "-2", "1").unwrap(), "x = 1");
        assert_eq!(
            cas_solve_quadratic("1", "0", "1").unwrap(),
            "no real solutions"
        );
        assert!(cas_simplify("").is_err());
        assert!(cas_differentiate("x", "").is_err());
        assert!(cas_solve_linear("0", "1").is_err());
        assert!(cas_solve_quadratic("0", "1", "1").is_err());
        assert!(cas_solve_quadratic("a", "1", "1").is_err());
    }

    #[test]
    fn physics_formulas_compute() {
        let run = |id: &str, a: &str, b: &str| physics_compute(id, a, b, 6).unwrap();
        assert_eq!(run("force", "2", "3"), ("6".to_owned(), "N"));
        assert_eq!(
            run("weight", "10", ""),
            (format_value(&Value::Float(10.0 * 9.80665), 6), "N")
        );
        assert_eq!(run("velocity", "100", "10"), ("10".to_owned(), "m/s"));
        assert_eq!(run("acceleration", "10", "2"), ("5".to_owned(), "m/s²"));
        assert_eq!(run("momentum", "2", "3"), ("6".to_owned(), "kg·m/s"));
        assert_eq!(run("kinetic_energy", "2", "3"), ("9".to_owned(), "J"));
        assert_eq!(
            run("potential_energy", "2", "10"),
            (format_value(&Value::Float(2.0 * 9.80665 * 10.0), 6), "J")
        );
        assert_eq!(run("voltage", "2", "3"), ("6".to_owned(), "V"));
        assert_eq!(run("current", "6", "3"), ("2".to_owned(), "A"));
        assert_eq!(run("resistance", "6", "2"), ("3".to_owned(), "Ω"));
        assert_eq!(run("power_vi", "6", "2"), ("12".to_owned(), "W"));
        assert_eq!(run("power_ri", "2", "3"), ("12".to_owned(), "W"));
        assert_eq!(run("power_vr", "6", "3"), ("12".to_owned(), "W"));
        assert!(physics_compute("velocity", "1", "0", 6).is_err());
        assert!(physics_compute("current", "1", "0", 6).is_err());
        assert!(physics_compute("force", "abc", "3", 6).is_err());
        assert!(physics_compute("nosuch", "1", "2", 6).is_err());
    }

    #[test]
    fn physics_constants_cover_si() {
        let names: Vec<&str> = physics_constants().iter().map(|c| c.name).collect();
        assert_eq!(names.len(), 6);
        assert!(names.contains(&"gravity"));
        assert!(names.contains(&"Planck"));
    }

    #[test]
    fn chemistry_frontend_flows() {
        let rep = chemistry_analyze("H2O").unwrap();
        assert!((rep.mass - 18.015).abs() < 1e-9);
        assert_eq!(rep.components.len(), 2);
        assert_eq!(rep.components[0].symbol, "H");
        assert!((rep.components.iter().map(|c| c.percent).sum::<f64>() - 100.0).abs() < 1e-9);
        assert!((chemistry_analyze("CO2").unwrap().mass - 44.009).abs() < 1e-9);
        assert!(chemistry_analyze("").is_err());
        assert!(chemistry_analyze("Xx2").is_err());
        let he = chemistry_element("He").unwrap();
        assert_eq!((he.name, he.number), ("Helium".to_owned(), 2));
        assert!(chemistry_element("Xx").is_none());
        assert_eq!(chemistry_table().len(), 118);
        assert_eq!(filter_elements("").len(), 118);
        assert_eq!(filter_elements("oxygen").len(), 1);
        assert_eq!(filter_elements("OXYGEN")[0].symbol, "O");
        assert!(filter_elements("nosuchelement_xyz").is_empty());
    }

    #[test]
    fn finance_frontend_flows() {
        assert!((finance_compound("1000", "5", "1").unwrap() - 1050.0).abs() < 1e-9);
        assert!(
            (finance_effective_rate("12", "12").unwrap() - 0.126_825_030_131_969_77).abs() < 1e-12
        );
        let (pmt, total, interest) = finance_loan("10000", "6", "12").unwrap();
        assert!((pmt - 860.66).abs() < 0.01);
        assert!((total - pmt * 12.0).abs() < 1e-9);
        assert!((interest - (total - 10_000.0)).abs() < 1e-9);
        assert!((finance_growth("100", "10", "2").unwrap() - 121.0).abs() < 1e-9);
        assert!((finance_growth_rate("100", "121", "2").unwrap() - 0.1).abs() < 1e-12);
        assert_eq!(finance_percent("200", "15").unwrap(), 30.0);
        assert_eq!(finance_percent_change("100", "120").unwrap(), 20.0);
        assert!(finance_compound("abc", "5", "1").is_err());
        assert!(finance_loan("1000", "5", "0").is_err());
        assert!(finance_percent_change("0", "5").is_err());
    }
}
