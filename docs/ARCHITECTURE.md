# Architecture

This document describes the **Phase 1 core engine foundation** and how the
crates fit together. Target audience: contributors and ODS-os integrators.

## 1. Crate map

```text
                    ┌──────────────┐
                    │  ocalc-cli   │  clap + rustyline, no logic
                    └──────┬───────┘
                           │ uses
┌──────────┐  ┌────────────▼───────────┐  ┌───────────────┐
│ocalc-num │◄─┤     ocalc-engine       ├─►│ ocalc-history │
│  -ber    │  │ lexer/parser/AST/eval  │  │ JSON store    │
└────┬─────┘  │ graph/expression/error │  └───────────────┘
     │        └─┬────────┬─────────┬───┘
     │          │        │         │
┌────▼────┐ ┌───▼───┐ ┌──▼────┐ ┌──▼────┐
│ocalc-   │ │ocalc- │ │ocalc- │ │ (std) │
│functions│ │modules│ │units  │ │ serde │
└─────────┘ └───────┘ └───────┘ └───────┘
```

| Crate | Responsibility | Key types |
|---|---|---|
| `ocalc-number` | Unified number system | `Number`, `NumberError` |
| `ocalc-functions` | Built-in math fns + constants | `FunctionRegistry`, `MathFunction`, `AngleMode`, `CONSTANTS` |
| `ocalc-modules` | Plugin traits + registry | `CalculatorModule`, `FunctionProvider`, `UnitProvider`, `ModuleRegistry`, `Capability` |
| `ocalc-units` | Unit conversion | `UnitConverter`, `CustomUnitRegistry`, `UnitCategory` |
| `ocalc-history` | Calculation log | `HistoryStore`, `HistoryEntry` |
| `ocalc-engine` | Pipeline + eval + graph | `Lexer`, `Parser`, `Expr`, `Calculator`, `Context`, `CalcError`, `Span` |
| `ocalc-cli` | UX only | clap `Cli`, rustyline REPL |

Dependency rule: **engine depends on everything; nothing depends on the
engine except the CLI/frontends.** Functions/modules/units/number/history
never import the engine. This keeps the core replaceable and testable.

## 2. Data flow

### 2.1 Single evaluation

```text
&str ──Lexer::tokenize──► Vec<Token{kind, span}>
     ──Parser::parse──► Expr{kind, span}
     ──eval(&Expr, &mut Context)──► Number
```

`Context` owns:
- `variables: HashMap<String, Number>` (assignments persist),
- `functions: FunctionRegistry` (built-ins, cloneable),
- `modules: ModuleRegistry` (plugins, `Arc`-backed, cloneable),
- `angle_mode: AngleMode`.

Name resolution order for `Call`: built-in registry (case-insensitive) →
plugin registry → `UnknownFunction`. For `Var`: variables → Greek aliases
(`π→pi`, `τ→tau`, `φ→phi`) → case-insensitive constants → `UnknownVariable`.

### 2.2 Reusable expressions

`Expression::parse(src)` stores `(source, ast)`. Graph sampling and future
plotters parse **once** then call `evaluate(&mut ctx)` per point with a fresh
variable scope. No re-lexing in hot loops.

### 2.3 Graph sampling

```text
sample_expression("sin(x)", "x", 0.0, 6.28, 101, &base)
  → Vec<GraphPoint { x: f64, y: f64, valid: bool }>
```

Each point clones `base` (variables + functions + `Arc` modules + angle mode),
binds `var = x`, evaluates. Per-point failure → `{ y: NaN, valid: false }`
so renderers can break polylines at `1/x`-style singularities. The engine
never draws.

## 3. Number system (`ocalc-number`)

```rust
pub enum Number { Integer(i64), BigInt(BigInt), Rational(BigRational),
                 Decimal(Decimal), Float(f64), Complex(Complex64) }
```

Promotion rank: `Integer(0) < BigInt(1) < Rational(2) < Decimal(3) < Float(4)
< Complex(5)`. Mixed ops promote both sides to the higher rank, then compute
in that domain — so `1 + 2.5` is exact decimal/float, never silently truncated.

Semantics worth knowing:
- `i64` overflow promotes to `BigInt` (never wraps, never panics).
- `int / int` is exact when divisible, else `Rational` (`1/3` stays exact
  until `to_f64()`).
- `Decimal` literals (`3.14`) preserve base-10 intent; `rust_decimal` with
  `maths` feature; checked ops map overflow to `NumberError::Overflow`.
- `Float` division by zero is an **error** (`DivisionByZero`), not `inf` —
  calculator semantics.
- `%` on complex/rational is rejected (`InvalidOperation`); bitwise/shifts
  require integer-valued operands (`NonIntegerBitwise` → engine `TypeError`).
- `pow`: integer exponents use exact repeated multiplication (negative →
  `Rational`); fractional exponents go through `f64`, negative base +
  fractional exponent yields `Complex`.
- `factorial`: non-negative integers only, `BigInt` accumulator, cap 10 000.
- `sqrt`/`ln` of negatives yield `Complex` instead of erroring.

## 4. Functions (`ocalc-functions`)

`FunctionRegistry: HashMap<String, MathFunction>` where `MathFunction` holds
`Arc<dyn Fn(&[Number], &FunctionContext)>`. Registry is `Clone` (shares `Arc`s).

- Trig (`sin cos tan asin acos atan atan2 sinh cosh tanh`) respects `AngleMode`.
  Direct trig converts input to radians; inverse trig converts output back.
- `log(x)` = base-10, `log(x, base)` arbitrary; `ln` delegates to
  `Number::ln` (complex fallback); `sqrt` delegates to `Number::sqrt`.
- Stats (`sum mean median variance stddev`) operate on `&[Number]`.
- `gcd`/`lcm` use Euclid on `BigInt` (need `Signed + Zero` traits in scope).
- Arity checked before dispatch (`Arity` error carries expected/got).

## 5. Plugins (`ocalc-modules`)

```rust
pub trait CalculatorModule: Send + Sync {
    fn metadata(&self) -> ModuleMetadata;
    fn init(&self) -> Result<(), String> { Ok(()) }
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> { None }
    fn as_unit_provider(&self) -> Option<&dyn UnitProvider> { None }
}
```

`ModuleRegistry` maps `id → Arc<dyn CalculatorModule>`, runs `init()` on
register, rejects duplicates, dispatches `call_function` to the first provider
returning `Some(..)`. `Capability::{Pure, Trigonometry, Statistics, Units,
History, Graphing, Custom}` lets hosts sandbox (e.g. allow only `Pure`).

The engine never names concrete modules — adding math means implementing
these traits in a new crate and calling `ctx.modules.register(...)`. See
`docs/MODULE_GUIDE.md`.

## 6. Units (`ocalc-units`)

Linear units store `factor_to_base`; temperature stores affine
`(factor, offset)` with base kelvin: `base = v*factor + offset`.
`UnitConverter` is case-insensitive, category-checked (`Length/Mass/
Temperature/Data/Energy/Time/Custom`). `CustomUnitRegistry` wraps it and
adds `register_custom("smoot", &["smoots"], "m", 1.7018)`.

## 7. History (`ocalc-history`)

`HistoryStore { entries, next_id, capacity }` — bounded (default 10 000, ring
eviction), `push/search/last/clear`, `to_json/from_json/save_to_file/
load_from_file` via serde. CLI persists after every eval. `next_id` is
monotonic across `clear()` so log references stay stable.

## 8. Errors

`CalcError` (engine) wraps `NumberError` + `FunctionError` via `From`, always
attaching the AST `Span`:

```rust
pub enum CalcError { InvalidToken{lexeme, span, message}, SyntaxError{span, message},
  MathError{message, span}, Overflow{message, span}, DivisionByZero{span, ..},
  UnknownFunction{name, span}, UnknownVariable{name, span},
  ArityError{name, expected, got, span}, TypeError{message, span} }
```

`Span{start, end}` is a byte-offset half-open range with `merge()` for
bounding boxes. `render_with_source(src)` appends a `^` caret line for CLI/IDE
display. Lexer errors suggest fixes (`did you mean '<<'?`).

## 9. Performance notes

- Lexer is a single-pass byte scanner, no regex.
- Parser is recursive descent, no backtracking, no allocation beyond AST.
- Hot paths (`Integer`, `Float`) avoid heap; `BigInt`/`Rational` only on demand.
- `pow` caps exact loops at 10 000 iterations, then falls back to float.
- Release profile: `opt-level=3, lto=true, codegen-units=1, panic=abort`.

## 10. Reserved extension points

- Matrix / equation / polynomial solvers: implement as `ocalc-matrix`,
  `ocalc-solve` crates exposing `FunctionProvider` (e.g. `det`, `solve`,
  `roots`) + new `Number` variants only if needed. No core edits required.
- Iced UI: depends on `ocalc-engine` (`Expression`, `sample_expression`,
  `CalcError::render_with_source`) + `ocalc-history`. Never on the CLI.
