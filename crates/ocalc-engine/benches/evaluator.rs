//! Evaluator benchmarks (Phase 3): end-to-end eval + reuse paths.
//!
//! Run with `cargo bench -p ocalc-engine`.

use std::time::Instant;

use ocalc_engine::{CalculatorEngine, Expression};

fn bench_eval(name: &str, src: &str, iters: usize) {
    let mut e = CalculatorEngine::new();
    for _ in 0..100 {
        let _ = e.eval(src);
    }
    let t = Instant::now();
    let mut ok = 0;
    for _ in 0..iters {
        if e.eval(src).is_ok() {
            ok += 1;
        }
    }
    let el = t.elapsed();
    println!(
        "eval/{name}: {iters} iters in {el:?} ({:.1} ns/op, ok={ok})",
        el.as_nanos() as f64 / iters as f64
    );
}

fn bench_reuse(name: &str, src: &str, iters: usize) {
    // Parse-once, evaluate-many (the graph / Iced hot path).
    let expr = Expression::parse_optimized(src).unwrap();
    let mut ctx = ocalc_engine::Context::new();
    ctx.set_var("x", ocalc_engine::Value::Float(0.5));
    for _ in 0..100 {
        let _ = expr.evaluate(&mut ctx);
    }
    let t = Instant::now();
    for _ in 0..iters {
        let _ = expr.evaluate(&mut ctx);
    }
    let el = t.elapsed();
    println!(
        "reuse/{name}: {iters} iters in {el:?} ({:.1} ns/op)",
        el.as_nanos() as f64 / iters as f64
    );
}

fn bench_graph(name: &str, src: &str, points: usize, iters: usize) {
    let e = CalculatorEngine::new();
    for _ in 0..5 {
        let _ = e.sample_graph(src, "x", 0.0, std::f64::consts::TAU, points);
    }
    let t = Instant::now();
    for _ in 0..iters {
        let _ = e.sample_graph(src, "x", 0.0, std::f64::consts::TAU, points);
    }
    let el = t.elapsed();
    println!(
        "graph/{name}x{points}: {iters} iters in {el:?} ({:.1} ms/sample-run)",
        el.as_millis() as f64 / iters as f64
    );
}

fn main() {
    println!("== ocalc evaluator benchmarks ==");
    bench_eval("arith", "1 + 2 * 3 - 4 / 2", 20_000);
    bench_eval("trig", "sin(0.5) + cos(0.5) + sqrt(25) * 10", 10_000);
    bench_eval(
        "stats",
        "mean(1,2,3,4,5,6,7,8,9,10) + sum(1,2,3,4,5)",
        10_000,
    );
    bench_eval("bigint", "2^40 + 20!", 5_000);
    bench_eval("vars", "x = 5", 10_000);

    bench_reuse("arith", "1 + 2 * 3", 50_000);
    bench_reuse("trig", "sin(x) + cos(x)", 20_000);

    bench_graph("sin", "sin(x)", 101, 200);
    bench_graph("poly", "x^3 - 2*x + 1", 1001, 50);
}
