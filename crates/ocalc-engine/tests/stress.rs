//! Stress tests (Phase 3): sustained load stays correct and bounded.
//!
//! Kept fast enough for `cargo test` (< ~5s debug). Heavier soak runs live
//! behind `#[ignore]` for nightly CI.

use ocalc_engine::{CalculatorEngine, EngineConfig, EngineLimits, Expression, Value};

#[test]
fn long_flat_sum() {
    let mut e = CalculatorEngine::new();
    let terms: Vec<String> = (1..=200).map(|i| i.to_string()).collect();
    let src = terms.join(" + ");
    // 1+…+200 = 20100.
    assert_eq!(e.eval(&src).unwrap().to_string(), "20100");
}

#[test]
fn thousand_sequential_evals() {
    let mut e = CalculatorEngine::new();
    for i in 0..1000 {
        let v = e.eval(&format!("{i} * 2 + 1")).unwrap();
        assert_eq!(v.to_string(), ((i * 2 + 1).to_string()));
    }
}

#[test]
fn hundred_variables() {
    let mut e = CalculatorEngine::new();
    for i in 0..100 {
        e.set_var(&format!("v{i}"), Value::Integer(i));
    }
    assert_eq!(e.variable_names().len(), 100);
    assert_eq!(e.eval("v0 + v99").unwrap().to_string(), "99");
    e.clear_vars();
    assert!(e.variable_names().is_empty());
}

#[test]
fn expression_reuse_ten_thousand_times() {
    let expr = Expression::parse("x * x + 2*x + 1").unwrap();
    let mut e = CalculatorEngine::new();
    for x in 0..10_000 {
        e.set_var("x", Value::Integer(x % 100));
        let _ = e.eval_expression(&expr).unwrap();
    }
}

#[test]
fn graph_ten_thousand_points() {
    let e = CalculatorEngine::new();
    let pts = e
        .sample_graph("sin(x)", "x", 0.0, std::f64::consts::TAU, 10_000)
        .unwrap();
    assert_eq!(pts.len(), 10_000);
    assert!(pts.iter().any(|p| p.valid));
}

#[test]
fn bigint_factorial_100() {
    let mut e = CalculatorEngine::new();
    let s = e.eval("100!").unwrap().to_string();
    assert_eq!(s.len(), 158);
    assert!(s.starts_with("933262154439"));
}

#[test]
fn nested_but_allowed_depth() {
    let mut e = CalculatorEngine::new();
    let mut src = "1".to_owned();
    for _ in 0..20 {
        src = format!("({src} + 1)");
    }
    assert_eq!(e.eval(&src).unwrap().to_string(), "21");
}

#[test]
fn variable_count_limit_enforced() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_variables: 4,
            ..EngineLimits::default()
        },
    };
    let mut ctx = ocalc_engine::Context::new();
    for i in 0..4 {
        ctx.set_var_limited(&format!("v{i}"), Value::Integer(i), &config.limits)
            .unwrap();
    }
    assert!(ctx
        .set_var_limited("overflow", Value::Integer(0), &config.limits)
        .is_err());
}

#[test]
#[ignore = "soak: 100k evals for nightly CI"]
fn soak_hundred_thousand_evals() {
    let mut e = CalculatorEngine::new();
    for i in 0..100_000 {
        let _ = e.eval(&format!("{i} % 7")).unwrap();
    }
}
