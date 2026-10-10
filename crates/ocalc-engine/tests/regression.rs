//! Regression tests (Phase 3): every fixed bug stays fixed.
//!
//! Each test names the original failure mode. Add a new case here (not in a
//! unit test) whenever a fuzzer or user report finds a crash/wrong-answer.

use ocalc_engine::{CalculatorEngine, Expression, Value};

// Promotion used to hit `unreachable!` when `from_bigint`/`from_rational`
// folded small values back to `Integer` mid-promotion.
#[test]
fn promotion_never_panics() {
    let mut e = CalculatorEngine::new();
    // Integer + BigInt, BigInt + Rational, Rational + Decimal, etc.
    for src in [
        "9223372036854775807 + 1",
        "9223372036854775807 + 1 + 1/3",
        "1 + 2.5",
        "1/3 + 0.5",
        "(2^100) + 1",
        "(2^100) + 0.5",
        "sqrt(-4) + 1",
        "1 + sqrt(-4)",
    ] {
        assert!(e.eval(src).is_ok(), "{src}");
    }
}

// `0 * x` must NOT fold to `0` (wrong for inf/NaN).
#[test]
fn zero_times_x_not_folded() {
    let opt = Expression::parse_optimized("0 * x").unwrap();
    assert!(
        matches!(opt.ast().kind, ocalc_engine::ExprKind::Binary { .. }),
        "{:?}",
        opt.ast()
    );
    // But `x + 0`, `x * 1`, `x^1`, `x^0` still fold.
    for (src, stays_binary) in [
        ("x + 0", false),
        ("x * 1", false),
        ("0 * x", true),
        ("x - x", true),
    ] {
        let o = Expression::parse_optimized(src).unwrap();
        assert_eq!(
            matches!(o.ast().kind, ocalc_engine::ExprKind::Binary { .. }),
            stays_binary,
            "{src}"
        );
    }
}

// `1/3` is exact Rational; tripling gives Integer(1), not 0.999...
#[test]
fn third_stays_exact() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("1/3 + 1/3 + 1/3").unwrap(), Value::Integer(1));
    assert_eq!(e.eval("1/2 + 1/3").unwrap().to_string(), "5/6");
}

// Decimal literals keep base-10 intent.
#[test]
fn decimal_exactness() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("0.1 + 0.2").unwrap().to_string(), "0.3");
}

// i64 overflow promotes to BigInt instead of wrapping.
#[test]
fn overflow_promotes() {
    let mut e = CalculatorEngine::new();
    let r = e.eval("2^100").unwrap();
    assert_eq!(r.to_string(), "1267650600228229401496703205376");
    assert!(matches!(r, Value::BigInt(_)));
    assert_eq!(
        e.eval("25!").unwrap().to_string(),
        "15511210043330985984000000"
    );
}

// Negative sqrt/ln yield Complex instead of erroring.
#[test]
fn negative_roots_are_complex() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("sqrt(-4)").unwrap().to_string(), "2i");
    assert!(e.eval("ln(-1)").unwrap().to_string().contains('i'));
}

// `1/0` is DivisionByZero (not inf), with spans.
#[test]
fn division_by_zero_is_error() {
    let mut e = CalculatorEngine::new();
    let err = e.eval("1/0").unwrap_err();
    assert!(matches!(
        err,
        ocalc_engine::CalcError::DivisionByZero { .. }
    ));
    assert!(err.render_with_source("1/0").contains('^'));
}

// Bitwise on floats is TypeError, not silent truncation.
#[test]
fn float_bitwise_rejected() {
    let mut e = CalculatorEngine::new();
    assert!(matches!(
        e.eval("1.5 AND 2").unwrap_err(),
        ocalc_engine::CalcError::TypeError { .. }
    ));
}

// `-2^2 == -(2^2)`, `3!^2 == (3!)^2`, `2^3^2 == 2^(3^2)`.
#[test]
fn precedence_corner_cases() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("-2^2").unwrap().to_f64(), -4.0);
    assert_eq!(e.eval("3!^2").unwrap().to_string(), "36");
    assert_eq!(e.eval("2^3^2").unwrap().to_f64(), 512.0);
    assert_eq!(e.eval("3!!").unwrap().to_string(), "720");
}

// `2^-3` parses (signed exponent), `**` is power.
#[test]
fn signed_exponent_and_starstar() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("2^-3").unwrap().to_f64(), 0.125);
    assert_eq!(e.eval("2 ** 3").unwrap().to_f64(), 8.0);
}

// Function names and constants are case-insensitive; Greek aliases work.
#[test]
fn names_case_insensitive() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("SIN(0)").unwrap().to_f64(), 0.0);
    assert!(e.eval("TRIPLE_PLACEHOLDER").is_err());
    assert!((e.eval("PI").unwrap().to_f64() - std::f64::consts::PI).abs() < 1e-12);
    assert!((e.eval("π").unwrap().to_f64() - std::f64::consts::PI).abs() < 1e-12);
}

// Module path serves functions when builtins are shadowed; unknown stays unknown.
#[test]
fn module_fallback_still_works() {
    use ocalc_engine::Context;
    let mut ctx = Context::empty();
    ctx.modules
        .register(std::sync::Arc::new(ocalc_functions::StatisticsModule::new()))
        .unwrap();
    let expr = ocalc_engine::Parser::parse_source("mean(1, 2, 3)").unwrap();
    assert_eq!(ocalc_engine::eval(&expr, &mut ctx).unwrap().to_f64(), 2.0);
}

// Graph singularities mark invalid instead of aborting.
#[test]
fn graph_singularity() {
    let e = CalculatorEngine::new();
    let pts = e.sample_graph("1/x", "x", -1.0, 1.0, 3).unwrap();
    assert!(!pts[1].valid);
}

// History ids stay monotonic across clear().
#[test]
fn history_ids_monotonic() {
    let mut h = ocalc_history::HistoryStore::new();
    h.push("1", "1");
    h.clear();
    assert_eq!(h.push("2", "2").id, 2);
}

// Optimizer is idempotent and span-preserving.
#[test]
fn optimizer_idempotent() {
    let once = ocalc_engine::optimize(ocalc_engine::Parser::parse_source("2 + 3 * 4 + x").unwrap());
    let twice = ocalc_engine::optimize(once.clone());
    assert_eq!(once.kind, twice.kind);
}

// `f(1,)` trailing comma is a syntax error with carets.
#[test]
fn trailing_comma_rejected() {
    let mut e = CalculatorEngine::new();
    let err = e.eval("max(1,)").unwrap_err();
    assert!(err.render_with_source("max(1,)").contains('^'));
}
