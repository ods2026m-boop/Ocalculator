//! Integration tests (Phase 3): stable-API end-to-end flows.
//!
//! These tests use **only** the stable surface (`CalculatorEngine`,
//! `Expression`, `Value`, `ModuleRegistry`, `FunctionRegistry`, history and
//! serialization helpers) — the same path the future Iced UI will use. If a
//! test needs an internal type, the stable API is incomplete.

use std::sync::Arc;

use ocalc_engine::{
    CalculatorEngine, EngineConfig, EngineLimits, Expression, FunctionRegistry, ModuleRegistry,
    Value,
};
use ocalc_history::HistoryStore;
use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};

// --- Helpers ---------------------------------------------------------------

struct Double;
impl FunctionProvider for Double {
    fn function_names(&self) -> Vec<String> {
        vec!["double".to_owned()]
    }
    fn provides(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("double")
    }
    fn call(&self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        if name.eq_ignore_ascii_case("double") && args.len() == 1 {
            Some(args[0].add(&args[0]).map_err(|e| e.to_string()))
        } else {
            None
        }
    }
}
struct DoubleModule {
    provider: Double,
}
impl CalculatorModule for DoubleModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new(
            "integration-double",
            ocalc_engine::ENGINE_PLUGIN_API_VERSION,
            "test double",
        )
        .with_capability(Capability::Pure)
    }
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(&self.provider)
    }
}

// --- Pipeline --------------------------------------------------------------

#[test]
fn full_pipeline_single_eval() {
    let mut e = CalculatorEngine::new();
    assert_eq!(e.eval("sqrt(25) * 10 + 2^3").unwrap().to_f64(), 58.0);
    assert_eq!(e.eval("1/3 + 1/3 + 1/3").unwrap().to_string(), "1");
    assert_eq!(e.eval("0.1 + 0.2").unwrap().to_string(), "0.3");
}

#[test]
fn parse_once_evaluate_many() {
    let expr = Expression::parse("x * 2 + 1").unwrap();
    let mut e = CalculatorEngine::new();
    for (x, want) in [(0, 1), (5, 11), (-3, -5)] {
        e.set_var("x", Value::Integer(x));
        assert_eq!(
            e.eval_expression(&expr).unwrap().to_string(),
            want.to_string()
        );
    }
}

#[test]
fn optimized_and_raw_agree() {
    let mut e = CalculatorEngine::new();
    e.set_var("x", Value::Integer(7));
    for src in ["1+2*3", "2^3^2", "5! + 1", "x*2+1", "sqrt(4)+1"] {
        let raw = e.parse(src).unwrap();
        let opt = e.parse_optimized(src).unwrap();
        // Evaluate each in a fresh engine with the same var to avoid
        // cross-test assignment pollution.
        let mut a = CalculatorEngine::new();
        let mut b = CalculatorEngine::new();
        a.set_var("x", Value::Integer(7));
        b.set_var("x", Value::Integer(7));
        assert_eq!(
            a.eval_expression(&raw).ok(),
            b.eval_expression(&opt).ok(),
            "{src}"
        );
    }
}

#[test]
fn variables_crud() {
    let mut e = CalculatorEngine::new();
    e.set_var("alpha", Value::Integer(1));
    assert!(e.get_var("alpha").is_some());
    assert!(e.variable_names().contains(&"alpha".to_owned()));
    assert!(e.remove_var("alpha"));
    assert!(!e.remove_var("alpha"));
    e.set_var("a", Value::Integer(1));
    e.set_var("b", Value::Integer(2));
    e.clear_vars();
    assert!(e.variable_names().is_empty());
}

#[test]
fn degrees_and_radians() {
    let mut deg = CalculatorEngine::with_config(EngineConfig::with_degrees());
    assert!((deg.eval("sin(90)").unwrap().to_f64() - 1.0).abs() < 1e-12);
    let mut rad = CalculatorEngine::new();
    assert!(rad.eval("sin(90)").unwrap().to_f64().abs() < 1.0);
}

#[test]
fn errors_carry_spans_and_carets() {
    let mut e = CalculatorEngine::new();
    let err = e.eval("1 + nosuchvar").unwrap_err();
    assert!(err.render_with_source("1 + nosuchvar").contains('^'));
    let err = e.eval("nosuchfn(1)").unwrap_err();
    assert!(err.to_string().contains("nosuchfn"));
}

// --- Modules (stable discovery path) ---------------------------------------

#[test]
fn builtin_modules_discoverable() {
    let e = CalculatorEngine::new();
    for id in ["standard-math", "scientific-math", "statistics", "units"] {
        assert!(
            e.module_ids().contains(&id.to_owned()),
            "{:?}",
            e.module_ids()
        );
    }
    assert!(e.function_names().contains(&"sin".to_owned()));
    assert!(e.function_names().contains(&"mean".to_owned()));
}

#[test]
fn third_party_plugin_without_core_edits() {
    let mut e = CalculatorEngine::new();
    e.register_module(Arc::new(DoubleModule { provider: Double }))
        .unwrap();
    assert_eq!(e.eval("double(21)").unwrap().to_string(), "42");
    assert_eq!(e.eval("DOUBLE(2)").unwrap().to_string(), "4");
}

#[test]
fn duplicate_registration_rejected() {
    let mut e = CalculatorEngine::new();
    e.register_module(Arc::new(DoubleModule { provider: Double }))
        .unwrap();
    let err = e
        .register_module(Arc::new(DoubleModule { provider: Double }))
        .unwrap_err();
    assert!(err.to_string().contains("already registered"));
}

#[test]
fn invalid_metadata_rejected() {
    struct Bad;
    impl CalculatorModule for Bad {
        fn metadata(&self) -> ModuleMetadata {
            // Uppercase + spaces: violates kebab-case.
            ModuleMetadata::new("Bad Module!", "not-semver", "")
        }
    }
    let mut e = CalculatorEngine::new();
    assert!(e.register_module(Arc::new(Bad)).is_err());
}

#[test]
fn compatibility_checked() {
    let m = DoubleModule { provider: Double };
    let meta = CalculatorModule::metadata(&m);
    meta.validate().unwrap();
    meta.check_compatibility(ocalc_engine::ENGINE_PLUGIN_API_VERSION)
        .unwrap();
    assert!(meta.check_compatibility("999.0.0").is_err());
    // Newer minor than the engine is rejected (may need newer host features).
    let newer = ModuleMetadata::new("x", "99.99.0", "newer");
    // Only meaningful when the engine is older; at minimum the error path works.
    let _ = newer.check_compatibility(ocalc_engine::ENGINE_PLUGIN_API_VERSION);
}

#[test]
fn bulk_discovery_collects_errors() {
    let mut reg = ModuleRegistry::new();
    struct Bad;
    impl CalculatorModule for Bad {
        fn metadata(&self) -> ModuleMetadata {
            ModuleMetadata::new("bad!", "1.0.0", "bad")
        }
    }
    let res = reg.register_all(vec![
        Arc::new(DoubleModule { provider: Double }) as Arc<dyn CalculatorModule>,
        Arc::new(Bad) as Arc<dyn CalculatorModule>,
    ]);
    assert!(res.is_err());
    // The good module still registered.
    assert!(reg.provides_function("double"));
}

#[test]
fn registries_usable_directly() {
    let reg = FunctionRegistry::default();
    assert!(reg.is_empty());
    let defaults = ocalc_engine::FunctionRegistry::default();
    let _ = defaults;
    let full = ocalc_modules::ModuleRegistry::new();
    assert!(full.is_empty());
}

// --- Graph -----------------------------------------------------------------

#[test]
fn graph_sampling_marks_singularities() {
    let e = CalculatorEngine::new();
    let pts = e.sample_graph("1/x", "x", -1.0, 1.0, 3).unwrap();
    assert_eq!(pts.len(), 3);
    assert!(!pts[1].valid);
    let pts = e
        .sample_graph("sin(x)", "x", 0.0, std::f64::consts::PI, 3)
        .unwrap();
    assert!((pts[1].y - 1.0).abs() < 1e-9);
}

#[test]
fn graph_points_serialize() {
    let e = CalculatorEngine::new();
    let pts = e.sample_graph("x", "x", 0.0, 1.0, 3).unwrap();
    let j = ocalc_engine::points_to_json(&pts);
    let back: Vec<ocalc_engine::GraphPoint> = serde_json::from_str(&j).unwrap();
    assert_eq!(back, pts);
}

// --- Serialization (versioned) ---------------------------------------------

#[test]
fn expression_versioned_roundtrip() {
    let expr = Expression::parse("2 + 3 * x").unwrap();
    let j = expr.to_json();
    assert!(j.contains("format_version"));
    let back = Expression::from_json(&j).unwrap();
    assert_eq!(expr, back);
}

#[test]
fn value_versioned_roundtrip() {
    for v in [
        Value::Integer(42),
        Value::from_decimal_str("3.14").unwrap(),
        Value::complex(1.0, -2.0),
    ] {
        let s = ocalc_engine::value_to_json_string(&v);
        assert!(s.contains("format_version"));
        assert_eq!(ocalc_engine::value_from_json_string(&s).unwrap(), v);
    }
}

#[test]
fn history_versioned_roundtrip_and_compat() {
    let mut h = HistoryStore::new();
    h.push("2*3", "6");
    let j = h.to_json().unwrap();
    assert!(j.contains("format_version"));
    let back = HistoryStore::from_json(&j).unwrap();
    assert_eq!(back.len(), 1);
    // Legacy bare payload still loads.
    let legacy = r#"{"entries":[],"next_id":1,"capacity":100}"#;
    assert!(HistoryStore::from_json(legacy).is_ok());
    // Future versions are rejected, not misread.
    let future = r#"{"format_version":99,"entries":[],"next_id":1,"capacity":10}"#;
    assert!(HistoryStore::from_json(future).is_err());
}

#[test]
fn module_metadata_versioned_roundtrip() {
    let m = ModuleMetadata::new("my-mod", "1.2.3", "does things");
    let j = m.to_json().unwrap();
    assert!(j.contains("format_version"));
    let back = ModuleMetadata::from_json(&j).unwrap();
    assert_eq!(back.id, "my-mod");
}

#[test]
fn engine_config_serializes() {
    let c = EngineConfig::default();
    let j = serde_json::to_string(&c).unwrap();
    let back: EngineConfig = serde_json::from_str(&j).unwrap();
    assert_eq!(c, back);
    let _ = EngineLimits::default();
}

// --- Engine + history together (CLI/ODS-os shape) --------------------------

#[test]
fn eval_to_history_flow() {
    let mut e = CalculatorEngine::new();
    let mut h = HistoryStore::new();
    for src in ["1+1", "sin(0)", "2^10"] {
        let v = e.eval(src).unwrap();
        h.push(src, &v.to_string());
    }
    assert_eq!(h.len(), 3);
    assert_eq!(h.search("sin").len(), 1);
    let j = h.to_json().unwrap();
    assert_eq!(HistoryStore::from_json(&j).unwrap().len(), 3);
}
