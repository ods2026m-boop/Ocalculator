# OCalc Engine

The reusable mathematical platform behind Ocalculator: a modular calculation
engine that powers the desktop app (future Iced UI), the `ocalc` CLI, ODS-os
scientific tools, and third-party modules. No UI lives here by design — the
engine computes values and graph points; frontends render them.

For the crate map and cross-crate data flow, see `ARCHITECTURE.md`. For the
plugin authoring tutorial, see `MODULE_GUIDE.md`. This file is the engine
operator's manual: pipeline, precision, extension points.

## 1. Architecture

```text
Input → Lexer → Tokens → Parser → AST → optimize → Evaluator → Number
                                          │              ├─ variables
                                          │              ├─ FunctionRegistry (angle-aware)
                                          │              └─ ModuleRegistry (plugins)
                                          │
y = f(x) → sample_expression → Vec<GraphPoint> (UI renders later)
```

Crates:

| Crate | Role |
|---|---|
| `ocalc-number` | `Number`: Integer/BigInt/Rational/Decimal/Float/Complex + promotion |
| `ocalc-functions` | `standard-math`, `scientific-math`, `statistics` registries + module wrappers |
| `ocalc-modules` | `CalculatorModule` / `FunctionProvider` / `UnitProvider` / `ModuleRegistry` / `Capability` |
| `ocalc-units` | Built-in converter + `units` module + custom-unit registry |
| `ocalc-matrix` | `Matrix`/`Vector` foundation (linear algebra substrate) |
| `ocalc-solver` | `Polynomial`, bisection/Newton, `A·x = b` |
| `ocalc-history` | Bounded searchable JSON history |
| `ocalc-engine` | Lexer/parser/AST/optimizer/evaluator/graph/errors |
| `ocalc-cli` | UX only (`eval`, REPL, `graph`, `units`, `history`, `functions`) |

Dependency rule: the engine depends on everything; nothing except frontends
depends on the engine. Functions/units/matrix/solver never import the engine.

## 2. Data flow

### 2.1 Single evaluation

```rust
use ocalc_engine::Calculator;
let mut calc = Calculator::new(); // built-ins + 4 plugin modules pre-registered
assert_eq!(calc.eval("1/3 + 1/3 + 1/3").unwrap().to_string(), "1");
```

1. `Lexer::tokenize` — single-pass byte scanner: decimal/float/scientific,
   `0x`/`0o`/`0b` radix ints, `_` separators, identifiers (incl. `π`),
   operators `+ - * / % ^ ** ! & | ~ << >> ( ) , =`, case-insensitive
   `AND OR XOR NOT MOD` folded to operators.
2. `Parser::parse_source` — recursive descent, precedence
   `= → OR → XOR → AND → shifts → + - → * / % → unary → ^ (right-assoc) → ! → primary`.
   Every node carries a byte `Span`; errors suggest fixes (`did you mean '<<'?`).
3. `optimize` — constant folding + safe identities (`x+0→x`, `x*1→x`,
   `-(-x)→x`). Only `Number`-only arithmetic folds; `1/0` and `(-1)!` are left
   for runtime errors; `Call` nodes never fold; `0*x→0` is *not* applied
   (`inf`/`NaN` hazard). `Calculator::eval` optimizes; `eval_expr` stays raw.
4. `eval` — variables → Greek aliases (`π→pi`) → constants → `UnknownVariable`;
   `Call` → built-in registry (angle-aware) → module registry → `UnknownFunction`.

Reusable path: `Expression::parse[_optimized]` once, then `evaluate(&mut ctx)`
per use. Graph sampling parses once and clones a fresh variable scope per point.

### 2.2 Variables, constants, calls

```text
x = 10        # assignment (right-assoc, persists in Context)
x * 5         # 50
pi e tau phi sqrt2 ...  # case-insensitive constants (+ π τ φ aliases)
sin(x) sqrt(x) log(x) mean(1,2,3) gcd(12,18)  # calls, case-insensitive
```

### 2.3 Graph sampling

```rust
use ocalc_engine::sample_expression;
let pts = sample_expression("sin(x)", "x", 0.0, 6.28, 101, calc.context()).unwrap();
// pts: Vec<GraphPoint { x, y, valid }> — valid=false at singularities (1/x at 0).
```

Per-point failures mark `valid: false` instead of aborting, so renderers break
polylines at discontinuities. `ModuleRegistry` is `Arc`-backed and cheaply
cloned per point, so plugins survive sampling.

## 3. Precision model (`ocalc-number`)

Rank: `Integer < BigInt < Rational < Decimal < Float < Complex`; mixed ops
promote to the higher rank. Promotion never normalizes (a past `unreachable!`
panic came from `from_bigint`/`from_rational` folding small values back to
`Integer` mid-promotion); normalization happens only on operation *results*.

- `1/3` is exact `Rational(1/3)`; `1/3+1/3+1/3 == Integer(1)`.
- `0.1+0.2 == Decimal("0.3")` (base-10 exact, 28–29 digits); the same in
  `Float` is inexact — by design.
- `i64` overflow promotes to arbitrary-precision `BigInt` (`2^100`, `25!` exact).
- `sqrt`/`ln` of negatives yield `Complex` instead of erroring.
- `Float` division by zero is `DivisionByZero`, not `inf` (calculator semantics).

Conversions: `to_f64` (complex → magnitude), `to_complex`, `to_rational`,
`to_decimal`, `as_bigint` (integer-valued only), `as_i64`, `kind()`,
`is_complex()`, plus `From` impls for `i64/i32/u64/i128/BigInt/BigRational/
Decimal/Complex64` and constructors `from_rational/rational/complex`.

## 4. Adding new functions

The engine never hardcodes names. Two paths, same implementations:

**Built-in path (angle-aware).** Add to the right category constructor in
`ocalc-functions/src/registry.rs` — `standard_registry()`,
`scientific_registry()`, or `statistics_registry()` — and it flows into
`default_registry()` automatically.

```rust
reg("cube", 1, Some(1), "cube a value", Arc::new(|a, _| {
    expect_arity("cube", a, 1)?;
    a[0].mul(&a[0]).and_then(|sq| sq.mul(&a[0])).map_err(FunctionError::from)
}));
```

**Plugin path (no core edits).** Implement `FunctionProvider` +
`CalculatorModule` (see `MODULE_GUIDE.md`), register at runtime:

```rust
calc.context_mut().register_module(Arc::new(MyModule::new())).unwrap();
assert_eq!(calc.eval("triple(14)").unwrap().to_string(), "42");
```

Note: `FunctionProvider::call` has no angle context, so trig served through
the *module* path evaluates in radians; the `FunctionRegistry` path is the
angle-aware one (`Calculator::with_degrees()`).

## 5. Adding new modules

Implement `CalculatorModule` (`metadata` + optional `as_function_provider` /
`as_unit_provider`); the uniform surface `name()/version()/functions()/
capabilities()` derives from metadata. Declare honest `Capability`s
(`Pure/Trigonometry/Statistics/Units/History/Graphing/Custom`) so hosts can
sandbox. Register with `ModuleRegistry::register` (runs `init()`, rejects
duplicates) or `Context::register_module`. Built-ins ship as modules:
`standard-math`, `scientific-math`, `statistics`, `units` — see
`ocalc-functions/src/modules.rs`, `ocalc-units/src/module.rs`.

Units without a module: `CustomUnitRegistry::register_custom("smoot",
&["smoots"], "m", 1.7018)`. Units *as* a module: `UnitsModule` (implements
`UnitProvider`; `ModuleRegistry::convert_unit` dispatches). Engine unit
*s syntax* (`1 km to m`) is a reserved future grammar item.

Inspect at runtime: `modules.module_ids()`, `modules.describe()`,
`modules.with_capability(&Capability::Pure)`.

## 6. Matrix & solver

- `ocalc-matrix`: dense row-major `Matrix`/`Vector` (`f64`): construction with
  shape validation, add/sub/mul, `mul_vec`, transpose, determinant (partial
  pivoting), Gauss-Jordan inverse, dot/norm/cross. `from_numbers` bridges
  exact `Number` scalars. Errors: `DimensionMismatch/NotSquare/Singular/
  IndexOutOfBounds/InvalidShape`.
- `ocalc-solver`: `Polynomial` (highest-first coeffs, Horner eval, derivative,
  exact degree ≤ 2 roots incl. complex: `x²-4=0 → ±2`, `x²+1=0 → ±i`),
  `bisection`/`newton` for higher-degree/transcendental equations,
  `solve_linear_system` (`A·x=b`, partial pivoting). Errors:
  `InvalidInput/NoSolution/NoConvergence/Linalg`.
- Engine integration (function providers like `det()`, `solve()`) is reserved
  future work — the crates are intentionally engine-independent today.

## 7. Errors

`CalcError`: `InvalidToken/SyntaxError/MathError/Overflow/DivisionByZero/
UnknownFunction/UnknownVariable/ArityError/TypeError`, each with an optional
`Span`. `render_with_source(src)` appends a caret line. Number/function errors
convert via `From` with spans attached at the call site. Optimizer fallible
folds are deferred to runtime precisely so these spans survive.

## 8. Testing & performance

`cargo test --workspace` (≈110 tests): lexer/parser edge cases (unclosed
parens, missing operands, `2^-3`, `3!!`, `**`), exactness (`1/3` roundtrip,
`0.1+0.2`, `2^100`), complex flows, module loading (empty-registry fallback,
third-party plugin without core edits), matrix (det/inverse/singular),
solver (`x²-4`, `±i`, bisection/Newton, 2×2 systems), history eviction/JSON.

Performance: single-pass lexer, backtracking-free parser, integer/float fast
paths, one-time parse + per-point scope clone for graphs, exact loops capped
(10 000) with float fallback. Release: `opt-level=3, lto, codegen-units=1`.
