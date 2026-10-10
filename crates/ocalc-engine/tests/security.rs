//! Parser security tests (Phase 3): fuzz + recursion/size guards.
//!
//! Goals: no input panics, hangs, or bypasses [`EngineLimits`]. Every case
//! must return `Ok` or a spanned [`CalcError`] — never unwind.

use ocalc_engine::{CalculatorEngine, EngineConfig, EngineLimits};

// --- Deterministic fuzzer (xorshift, no new deps) ---------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a>(&mut self, table: &'a [&'a str]) -> &'a str {
        table[self.below(table.len())]
    }
}

const PIECES: &[&str] = &[
    "1",
    "2",
    "3.14",
    "0xFF",
    "0b1",
    "x",
    "sin",
    "(",
    ")",
    "+",
    "-",
    "*",
    "/",
    "%",
    "^",
    "**",
    "!",
    "&",
    "|",
    "~",
    "<<",
    ">>",
    ",",
    "=",
    " ",
    "AND",
    "OR",
    "XOR",
    "NOT",
    "MOD",
    "pi",
    "e",
    "π",
    ".",
    "e10",
    "0",
    "9999999999999999999999",
    "\"",
    "@",
    "#",
    "$",
    "\n",
    "\t",
    "NaN",
    "inf",
];

fn fuzz_one(rng: &mut Rng) -> String {
    let n = 1 + rng.below(12);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(PIECES));
    }
    s
}

#[test]
fn fuzz_never_panics() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut engine = CalculatorEngine::new();
    for _ in 0..10_000 {
        let src = fuzz_one(&mut rng);
        // Must not panic; any Result is acceptable.
        let _ = engine.eval(&src);
        // Parsing alone must also be panic-free.
        let _ = ocalc_engine::Expression::parse(&src);
    }
}

#[test]
fn fuzz_targeted_adversarial_cases() {
    let mut e = CalculatorEngine::new();
    let cases = [
        "",
        "   ",
        "@",
        "1 2 3",
        "(((((1",
        "))))",
        "1 + * 3",
        "* 3",
        "f(",
        "f()",
        "f(1,)",
        "(,)",
        "2 ^^ 3",
        "x = = 3",
        "1 = 2",
        ".",
        "1e",
        "0x",
        "0b2",
        "'",
        "\"quote\"",
        "πππ",
        "\0",
        "1\u{0}2",
        &"(".repeat(500),
        &")".repeat(500),
        &"1+".repeat(1000),
        &"-".repeat(500),
        &"!".repeat(100),
        &"~".repeat(200),
        "999999999999999999999999999999999999999^999999999",
        "1000000!",
        "1 << 1000000000",
        "sin(1,2,3,4,5,6,7,8,9,10,11,12)",
        &format!(
            "max({})",
            (0..500)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
        "((((1))))",
        "---5",
        "NOT NOT 5",
    ];
    for src in cases {
        let r = e.eval(src);
        // Either outcome is fine; the test fails only on panic (which would
        // abort the harness) or on success with an absurd value — spot-check
        // that huge shifts/factorials error instead.
        if src == "1000000!" || src.contains("1 << 1000000000") {
            assert!(r.is_err(), "{src:?} must be rejected, got {r:?}");
        }
    }
}

// --- Limits ----------------------------------------------------------------

#[test]
fn expression_too_long_rejected() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_expression_len: 8,
            ..EngineLimits::default()
        },
    };
    let mut e = CalculatorEngine::with_config(config);
    let err = e.eval("123456789").unwrap_err();
    assert!(err.is_limit(), "got {err}");
    assert!(err.render_with_source("123456789").contains('^'));
}

#[test]
fn token_flood_rejected() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_tokens: 16,
            ..EngineLimits::default()
        },
    };
    let mut e = CalculatorEngine::with_config(config);
    let src = (0..100)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    assert!(e.eval(&src).unwrap_err().is_limit());
}

#[test]
fn recursion_depth_rejected_safely() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_recursion_depth: 8,
            ..EngineLimits::default()
        },
    };
    let mut e = CalculatorEngine::with_config(config);
    let deep = "((((((((((1))))))))))"; // 10 parens > 8
    let err = e.eval(deep).unwrap_err();
    assert!(err.is_limit(), "got {err}");
}

#[test]
fn default_recursion_guard_blocks_deep_attack() {
    let mut e = CalculatorEngine::new();
    let attack = "(".repeat(500) + &")".repeat(500);
    assert!(e.eval(&attack).is_err());
}

#[test]
fn too_many_args_rejected() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_function_args: 4,
            ..EngineLimits::default()
        },
    };
    let mut e = CalculatorEngine::with_config(config);
    assert!(e.eval("max(1,2,3,4,5)").unwrap_err().is_limit());
}

#[test]
fn graph_point_cap_enforced() {
    let config = EngineConfig {
        angle_mode: ocalc_engine::AngleMode::Radians,
        limits: EngineLimits {
            max_graph_points: 10,
            ..EngineLimits::default()
        },
    };
    let e = CalculatorEngine::with_config(config);
    assert!(e.sample_graph("x", "x", 0.0, 1.0, 11).is_err());
    assert!(e.sample_graph("x", "x", 0.0, 1.0, 10).is_ok());
}

#[test]
fn eval_depth_guard() {
    // Programmatic deep AST (bypasses the parser) still guarded.
    let mut deep = ocalc_engine::Expr::new(
        ocalc_engine::ExprKind::Number(ocalc_engine::Value::Integer(1)),
        ocalc_engine::Span::point(0),
    );
    for _ in 0..50 {
        deep = ocalc_engine::Expr::new(
            ocalc_engine::ExprKind::Unary {
                op: ocalc_engine::UnOp::Neg,
                expr: Box::new(deep),
            },
            ocalc_engine::Span::point(0),
        );
    }
    let mut ctx = ocalc_engine::Context::new();
    assert!(ocalc_engine::eval_limited(&deep, &mut ctx, 8).is_err());
    assert!(ocalc_engine::eval_limited(&deep, &mut ctx, 10_000).is_ok());
}

#[test]
fn sandbox_config_is_strict() {
    let sandbox = EngineConfig::sandbox();
    assert!(sandbox.limits.max_expression_len < EngineLimits::default().max_expression_len);
    assert!(sandbox.limits.max_graph_points < EngineLimits::default().max_graph_points);
    sandbox.validate().unwrap();
    let mut e = CalculatorEngine::with_config(sandbox);
    // 5k-char expression exceeds the 4k sandbox cap.
    let big = "1+".repeat(2500) + "1";
    assert!(e.eval(&big).unwrap_err().is_limit());
}

#[test]
fn degenerate_limits_rejected() {
    let bad = EngineLimits {
        max_tokens: 0,
        ..EngineLimits::default()
    };
    assert!(bad.validate().is_err());
    let bad = EngineLimits {
        max_graph_points: usize::MAX,
        ..EngineLimits::default()
    };
    assert!(bad.validate().is_err());
}
