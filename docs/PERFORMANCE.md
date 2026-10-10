# OCalc Performance (Phase 3)

Targets: sub-microsecond arithmetic evals, sub-2µs parse of typical inputs,
10k-point graph sweeps in milliseconds, zero-allocation integer/float fast
paths. Release profile: `opt-level=3, lto=true, codegen-units=1,
panic=abort` (see workspace `Cargo.toml`).

## 1. Running the benchmarks

No external harness (stable-safe, `harness = false` plain `main`s):

```bash
cargo bench -p ocalc-engine    # parser + evaluator benches
cargo bench -p ocalc-matrix    # matrix benches
cargo bench --workspace        # everything
```

Each bench warms up, then prints `iters`, wall time and `ns/op`:

```text
== ocalc parser benchmarks ==
lexer/small: 20000 iters in 3.7ms (186.1 ns/op, toks=120000)
parser/small: 20000 iters in 7.5ms (373.7 ns/op, ok=20000)
parser/medium: 10000 iters in 18.1ms (1809.1 ns/op, ok=10000)
...
== ocalc evaluator benchmarks ==
eval/arith: 20000 iters in 14.8ms (740.9 ns/op, ok=20000)
eval/trig: 10000 iters in 13.8ms (1383.7 ns/op, ok=10000)
reuse/arith: 50000 iters in 166µs (3.3 ns/op)      # folded to a constant
graph/sinx101: 200 iters in 1.9ms (0.0 ms/sample-run)
...
== ocalc matrix benchmarks ==
matrix/mul3x3: 20000 iters in 832µs (41.6 ns/op)
matrix/det3x3: 20000 iters in 678µs (33.9 ns/op)
```

Reference machine (dev container, release): typical arithmetic eval ~0.7µs,
trig eval ~1.4µs, stats-10 ~3.6µs, `sin(x)` 101-point sweep ~10µs total.
Treat these as order-of-magnitude guides; track regressions relatively in CI
logs rather than asserting absolute thresholds.

Bench inventory:

| Bench | File | What it measures |
|---|---|---|
| `lexer/*` | `ocalc-engine/benches/parser.rs` | tokenize small/medium/200-term chains |
| `parser/*` | same | full parse: precedence, trig, bigint, long chains, 40-deep nesting |
| `eval/*` | `ocalc-engine/benches/evaluator.rs` | one-shot `eval`: arith/trig/stats/bigint/assign |
| `reuse/*` | same | parse-once → `evaluate` many (Iced hot path) |
| `graph/*` | same | 101-pt `sin`, 1001-pt cubic sweeps |
| `matrix/*` | `ocalc-matrix/benches/matrix.rs` | 3×3 mul/det/inverse/transpose, 10×10 mul/det |

## 2. What Phase 3 optimized

1. **Integer/float fast paths** (`ocalc-number/src/ops.rs`): `add/sub/mul/div/rem`
   match `(Integer, Integer)` / `(Float, Float)` *before* `promote_pair`, so
   hot arithmetic never clones heap payloads or allocates promotion temporaries.
   Mixed ranks still promote (higher rank wins) exactly as before.
2. **Single-probe dispatch** (`ocalc-engine/src/evaluator.rs`): built-ins do one
   `FunctionRegistry::call` (the `Unknown` arm falls through to modules) instead
   of `contains` + `call`; modules do one `call_function` scan instead of
   `provides_function` + `call_function`. Halves hashing and lowercase
   allocations on the hot path.
3. **Cheap provider hooks** (`ocalc-modules`): `FunctionProvider::provides` /
   `UnitProvider::provides` default to a case-insensitive scan but can be
   overridden to avoid the `Vec<String>` allocation per call. The registry uses
   these hooks throughout.
4. **Parser allocation trims** (`lexer.rs`, `parser.rs`): lexer pre-reserves
   `len/3` token capacity; `parse_assign` / `parse_primary` match by reference
   and clone payloads only on the taken branch (no `Number`/`String` clone per
   precedence level).
5. **Graph reuse** (`graph.rs`): parse + fold once, then reuse one scratch
   `Context` per sweep (`clone_from` the base variables + sweep binding) instead
   of cloning the whole context per point. Assignments still never leak across
   points (scope restored each iteration). Functions/modules are `Arc`-shared.
6. **History eviction** (`ocalc-history`): bulk `drain` instead of per-push
   `remove(0)` (O(n) → amortized O(1) under sustained logging).
7. **Optimizer as benchmark**: `reuse/arith` at ~3ns/op is the constant-folding
   dividend — `1+2*3` evaluates as a single cloned `Integer`.

Deliberately *not* changed: promotion semantics, `0*x` non-folding
(NaN hazard), fallible-fold deferral (span fidelity), `Decimal`/`Rational`
exactness. Performance never trades correctness.

## 3. Hot-path guide

```text
eval("1 + 2")  →  check_source_len (memchr-free len check)
              →  Lexer::tokenize_limited (1 pass, no regex, pre-reserved Vec)
              →  Parser::parse_source_limited (backtracking-free, depth-counted)
              →  optimize (1 bottom-up pass, Number-only folds)
              →  eval_limited (Integer fast path, single-probe call)
```

- Lexer: single-pass byte scanner; radix/decimal/float branches inline.
- Parser: recursive descent, iterative binary loops, recursion only for
  nesting (parens/calls/unary/assign-RHS) with a depth counter.
- Evaluator: variables are one `HashMap` lookup; constants are a static-slice
  scan; calls are one registry hash + one module scan.
- Graph: 1 parse + N cheap evals; per-point cost ≈ one `set_var` + eval.

## 4. Profiling

```bash
# Flamegraph-ish (Linux perf):
cargo build --release -p ocalc-cli
perf record -g ./target/release/ocalc eval "sum(1,2,3,4,5,6,7,8,9,10)"
perf report

# Focused Criterion-style loop without new deps:
cargo bench -p ocalc-engine --bench evaluator
```

Suspect list when a profile looks off: `function_names()` allocations (override
`provides`), `to_f64()` on `Decimal`/`Rational` in tight loops (prefer integer
paths), `variables.clone()` in custom samplers (use `clone_from` like
`sample_expression_limited`), `format!` in error paths (cold, fine).

## 5. Performance contracts

- Interactive evals (`< 100` tokens) complete in microseconds, never milliseconds.
- `Expression::parse` once + `evaluate` N times is ≥10× cheaper than N× `eval`
  for repeated sweeps (see `reuse/*` vs `eval/*`).
- Graph cost scales linearly in points; 10k-point `sin` sweep < 10ms release.
- Limits (`EngineLimits`) bound worst-case work: 8k tokens / 128 nesting / 128
  args / 100k points by default, tighter in sandboxes. Hostile inputs fail fast
  with `LimitExceeded`, never hang (see `tests/security.rs`, `tests/stress.rs`).
