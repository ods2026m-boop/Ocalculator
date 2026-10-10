# OCalc Stable API

This is the contract every frontend depends on: the Iced desktop app, the
`ocalc` CLI, ODS-os tools, and future applications. If you render results or
sample graphs, depend **only** on the types in §1. Everything else is internal
detail that may change without a major bump.

For the pipeline internals see `ENGINE.md`; for the crate map see
`ARCHITECTURE.md`. For plugin authoring see `PLUGIN_SYSTEM.md` (+ the
`MODULE_GUIDE.md` tutorial). For numbers/throughput see `PERFORMANCE.md`.

## 1. Stable surface

```rust
use ocalc_engine::{
    // Guarded eval (the only evaluator a UI needs):
    CalculatorEngine, EngineConfig, EngineLimits,
    // Reusable expressions + exact values:
    Expression, Value, ValueExt,
    // Plugins:
    ModuleRegistry, FunctionRegistry, CalculatorModule, FunctionProvider,
    UnitProvider, ModuleMetadata, Capability, ENGINE_PLUGIN_API_VERSION,
    // Errors + diagnostics:
    CalcError, Span,
    // Headless sampling (UI renders the points):
    GraphPoint, sample_expression, sample_expression_limited, points_to_json,
    // Trig mode:
    AngleMode,
};
```

Lower-level items (`Calculator`, `Context`, `Lexer`, `Parser`, `Expr`,
`BinOp`, `UnOp`, `Token`, `optimize`, `eval`) remain public for advanced
embedding and tests, but UI code must not need them.

## 2. `CalculatorEngine`

```rust
use ocalc_engine::{CalculatorEngine, EngineConfig};

let mut engine = CalculatorEngine::new();               // radians + defaults
let mut degrees = CalculatorEngine::with_degrees();     // degrees + defaults
let mut sandbox = CalculatorEngine::sandboxed();        // tight limits
let mut custom = CalculatorEngine::with_config(EngineConfig {
    angle_mode: ocalc_engine::AngleMode::Degrees,
    limits: ocalc_engine::EngineLimits { max_graph_points: 5_000, ..Default::default() },
});

// One-shot:
let v = engine.eval("sqrt(25) * 10 + 2^3")?;            // Value == 58
// Parse-once, evaluate-many (highlighting, tables, graph sweeps):
let expr = engine.parse("x * 2 + 1")?;
engine.set_var("x", ocalc_engine::Value::Integer(5));
let v = engine.eval_expression(&expr)?;                 // 11
// Optimized variant folds `2*3` once instead of per evaluation:
let opt = engine.parse_optimized("2*3 + sin(x)")?;

// Variables:
engine.set_var("x", value);
engine.get_var("x");
engine.variable_names();   // sorted, for inspectors
engine.remove_var("x");
engine.clear_vars();

// Modules:
engine.register_module(std::sync::Arc::new(MyModule::new()))?;
engine.register_module_checked(module, ocalc_engine::ENGINE_PLUGIN_API_VERSION)?;
engine.module_ids();
engine.function_names();   // built-ins + plugins, sorted + deduped

// Graph:
let pts = engine.sample_graph("sin(x)", "x", 0.0, 6.28, 101)?;

// Config:
engine.set_angle_mode(ocalc_engine::AngleMode::Degrees);
engine.set_config(new_config)?;   // validates limits
```

Semantics:

- `eval` = `check_source_len` → `Parser::parse_source_limited` →
  `optimize` → depth-guarded `eval_limited`. Optimization only folds
  `Number`-only arithmetic and safe identities; fallible folds (`1/0`) and
  `Call` nodes are left for runtime so error spans point at the source.
- Assignments (`x = 5`) persist in the engine's context. Graph sampling clones
  an isolated scope per point, so `y = x*2` inside a sampled expression never
  leaks across points.
- Name resolution: variables → Greek aliases (`π→pi`) → case-insensitive
  constants → `UnknownVariable`; calls: built-in `FunctionRegistry`
  (angle-aware) → `ModuleRegistry` → `UnknownFunction`.
- Variable-count overflow reports `LimitExceeded` (see §6).

## 3. `Expression`

```rust
let raw = Expression::parse("2 + 3 * x")?;              // spans preserved
let opt = Expression::parse_optimized("2 + 3 * x")?;    // folded
let guarded = Expression::parse_limited(src, &limits)?;
let rebuilt = Expression::from_ast(src, ast);

expr.source();          // original text
expr.ast();             // &Expr for highlighting
expr.format_version();  // always 1 (v1)
expr.optimize();        // in place, idempotent
expr.into_optimized();  // consuming variant
expr.evaluate(&mut ctx)?;
expr.evaluate_limited(&mut ctx, max_depth)?;
Expression::evaluate_fresh("1+1")?;  // one-shot with temp context
```

Serialization (versioned, §7):

```rust
let json = expr.to_json();              // {"format_version":1,"source":…,"ast":…}
let back = Expression::from_json(&json)?;
// Source-only payloads re-parse: {"source": "1 + 2"}.
// Unknown future format_versions are rejected, not misread.
```

## 4. `Value`

`pub type Value = ocalc_number::Number` — one exact representation shared by
the engine and all modules:

| Variant | Meaning |
|---|---|
| `Integer(i64)` | fast path |
| `BigInt` | arbitrary precision (`2^100`, `25!` exact) |
| `Rational` | exact `1/3`; `1/3+1/3+1/3 == 1` |
| `Decimal` | base-10 exact (`0.1+0.2 == 0.3`) |
| `Float(f64)` | IEEE-754 (`to_f64`, `sqrt`/`pow` fallbacks) |
| `Complex` | `sqrt(-4) → 2i`, `ln(-1) → πi` |

Helpers:

```rust
value.kind();              // "integer" | "bigint" | …
value.to_f64();            // complex → magnitude
value.to_string();         // exact display for result panes
ocalc_engine::value_kind_name(&v);
v.to_display_f64();        // ValueExt
v.to_result_string();      // ValueExt

let s = ocalc_engine::value_to_json_string(&v);   // {"format_version":1,"value":…}
let back = ocalc_engine::value_from_json_string(&s)?;
```

Wire shape is `{"kind":"bigint","value":"1267…"}` (strings for
arbitrary-precision variants, `{"re":…,"im":…}` for complex, `"NaN"`/`"inf"`
strings for non-finite floats since JSON numbers cannot express them).
`VALUE_FORMAT_VERSION == 1`; the `kind` tags are frozen.

## 5. Registries

```rust
// Built-ins (angle-aware fast path):
let mut functions = FunctionRegistry::new();
functions.register(MathFunction::new("cube", 1, Some(1), "cube", func));
functions.call("SIN", &args, &ctx)?;   // case-insensitive
functions.names();                     // sorted

// Plugins (see PLUGIN_SYSTEM.md):
let mut modules = ModuleRegistry::new();
modules.register(module)?;             // validates metadata + runs init()
modules.register_checked(module, ENGINE_PLUGIN_API_VERSION)?;  // + compat
modules.register_all(vec![a, b])?;     // bulk discovery; collects errors
modules.validate_all();                // Vec<ModuleError>
modules.check_compatibility_all("1.0.0");
modules.provides_function("double");
modules.call_function("double", &args)?;
modules.module_ids();                  // sorted
modules.describe();                    // (name, version, functions, caps)
modules.with_capability(&Capability::Pure);
```

The evaluator never hardcodes names: built-ins first (single hash lookup, no
prior `contains` probe), then one module-dispatch scan (no prior `provides`
probe). `provides_function` uses the cheap `FunctionProvider::provides` hook
so high-cardinality providers can avoid a `Vec` allocation per call.

## 6. Errors and limits

`CalcError` (all with optional `Span`, all JSON-serializable):

`InvalidToken | SyntaxError | MathError | Overflow | DivisionByZero |
UnknownFunction | UnknownVariable | ArityError | TypeError | LimitExceeded`

```rust
err.render_with_source(src);  // appends `^` caret line
err.with_span(span);
err.is_limit();
CalcError::limit("…");
```

`EngineLimits` (defaults / sandbox):

| Limit | Default | Sandbox |
|---|---|---|
| `max_expression_len` | 32 768 B | 4 096 B |
| `max_tokens` | 8 192 | 1 024 |
| `max_recursion_depth` | 128 | 32 |
| `max_eval_depth` | 1 024 | 64 |
| `max_function_args` | 128 | 32 |
| `max_graph_points` | 100 000 | 10 000 |
| `max_variables` | 1 024 | 128 |

`EngineConfig { angle_mode, limits }` serializes to JSON. `validate()` rejects
zero/absurd caps. `check_source_len / check_token_count / check_arg_count /
check_graph_points` are the enforcement points (lexer → parser → evaluator →
graph). Sandbox with `EngineConfig::sandbox()` / `CalculatorEngine::sandboxed()`
for untrusted input. See `tests/security.rs` for the adversarial corpus.

## 7. Serialization versions

| Payload | Version const | Envelope |
|---|---|---|
| `Expression` | `EXPRESSION_FORMAT_VERSION = 1` | `{"format_version":1,"source":…,"ast":…}` |
| `Value` | `VALUE_FORMAT_VERSION = 1` | `{"format_version":1,"value":…}` |
| `HistoryStore` | `HISTORY_FORMAT_VERSION = 1` | `{"format_version":1,"capacity":…,"next_id":…,"entries":[…]}` |
| `ModuleMetadata` | `MODULE_METADATA_FORMAT_VERSION = 1` | `{"format_version":1,"metadata":{…}}` |
| `EngineConfig` | (inline, no envelope) | `{"angle_mode":…,"limits":{…}}` |

Rules: loaders accept the current version plus documented legacy shapes
(bare history stores, bare value objects, source-only expressions) and reject
unknown future versions with an explicit error. Never silently misread.

History files persist atomically (temp + rename) via
`HistoryStore::save_to_file`; `load_from_file` repairs overfilled capacities
and non-monotonic ids from hand-edited files.

## 8. Graph

```rust
let pts: Vec<GraphPoint> = sample_expression("sin(x)", "x", 0.0, 6.28, 101, ctx)?;
let pts = sample_expression_limited(src, var, from, to, n, ctx, &limits)?;
let json = points_to_json(&pts);   // Vec<GraphPoint> is Serialize/Deserialize
```

`GraphPoint { x, y, valid }` — per-point failures and non-finite `y` yield
`valid: false` (renderer breaks the polyline) instead of aborting. Sampling
parses once, folds constants once, then reuses one scratch context per point
(`clone_from` the base variables + sweep binding; assignments never leak).

## 9. Frontend checklist

- [ ] Depend on `ocalc-engine` (+ `ocalc-history` for the log pane) only.
- [ ] One `CalculatorEngine` per session; `parse` once per input line, `eval`
  or `eval_expression` per submit.
- [ ] Render `Value::to_string()` in result panes, `to_f64()` for plots.
- [ ] Show `render_with_source(input)` on errors; treat `is_limit()` like a
  syntax error (user-fixable, with the violated limit in the message).
- [ ] Persist history with `to_json`/`from_json`; persist pinned expressions
  with `Expression::to_json`.
- [ ] Gate untrusted input with `EngineConfig::sandbox()` and strict
  `register_module_checked`.
