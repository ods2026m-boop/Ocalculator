//! Parser benchmarks (Phase 3): lexer + parser throughput.
//!
//! Run with `cargo bench -p ocalc-engine` (harness = false, stable-safe).
//! Each benchmark prints iterations, total time and ns/op so regressions are
//! visible in CI logs without extra tooling.

use std::time::Instant;

use ocalc_engine::{Lexer, Parser};

fn bench_case(name: &str, src: &str, iters: usize) {
    // Warmup.
    for _ in 0..100 {
        let _ = Parser::parse_source(src);
    }
    let t = Instant::now();
    let mut ok = 0;
    for _ in 0..iters {
        if Parser::parse_source(src).is_ok() {
            ok += 1;
        }
    }
    let el = t.elapsed();
    let ns = el.as_nanos() as f64 / iters as f64;
    println!("parser/{name}: {iters} iters in {el:?} ({ns:.1} ns/op, ok={ok})");
}

fn bench_lexer(name: &str, src: &str, iters: usize) {
    for _ in 0..100 {
        let _ = Lexer::new(src).tokenize();
    }
    let t = Instant::now();
    let mut toks = 0;
    for _ in 0..iters {
        if let Ok(v) = Lexer::new(src).tokenize() {
            toks += v.len();
        }
    }
    let el = t.elapsed();
    let ns = el.as_nanos() as f64 / iters as f64;
    println!("lexer/{name}: {iters} iters in {el:?} ({ns:.1} ns/op, toks={toks})");
}

fn main() {
    println!("== ocalc parser benchmarks ==");
    let small = "1 + 2 * 3";
    let medium = "sin(x) + sqrt(25) * 10 + 2^3 + max(1, min(2, 3))";
    let trig = "sin(0.5) + cos(0.5) + tan(0.25) + asin(0.5) + log(100, 10)";
    let bigint = "2^100 + 25! + 0xFF * 0b1010 + 123456789 * 987654321";
    // Long flat chain (token-limit stress, still under defaults).
    let long_terms: Vec<String> = (0..200).map(|i| i.to_string()).collect();
    let long = long_terms.join(" + ");
    // Deep-ish nesting (within the 128 guard).
    let mut nested = "1".to_owned();
    for _ in 0..40 {
        nested = format!("({nested} + 1)");
    }

    bench_lexer("small", small, 20_000);
    bench_lexer("medium", medium, 10_000);
    bench_lexer("long200", &long, 2_000);

    bench_case("small", small, 20_000);
    bench_case("medium", medium, 10_000);
    bench_case("trig", trig, 10_000);
    bench_case("bigint", bigint, 5_000);
    bench_case("long200", &long, 1_000);
    bench_case("nested40", &nested, 5_000);
}
