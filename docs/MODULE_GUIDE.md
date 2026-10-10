# Module Development Guide

You can extend Ocalculator **without modifying the core engine**. Implement
one of the traits in `ocalc-modules` and register it — the evaluator picks it
up automatically.

## 1. Which trait?

| Goal | Implement |
|---|---|
| New function `foo(x, …)` | `FunctionProvider` (+ `CalculatorModule`) |
| New units / conversions | `UnitProvider` (+ `CalculatorModule`) |
| Both / metadata / init logic | `CalculatorModule` with both hooks |

```rust
use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

pub struct Double;
impl FunctionProvider for Double {
    fn function_names(&self) -> Vec<String> { vec!["double".into()] }
    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        if name == "double" && args.len() == 1 {
            Some(args[0].add(&args[0]).map_err(|e| e.to_string()))
        } else { None } // not mine → registry tries next module
    }
}

pub struct DoubleModule { provider: Double }
impl CalculatorModule for DoubleModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new("demo-double", "1.0.0", "doubles a value")
            .with_capability(Capability::Pure)
    }
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(&self.provider) // must borrow owned state, never a temporary
    }
}
```

Register at startup (CLI, app, or test):

```rust
use std::sync::Arc;
use ocalc_engine::Calculator;
let mut calc = Calculator::new();
calc.context_mut().modules.register(Arc::new(DoubleModule { provider: Double })).unwrap();
assert_eq!(calc.eval("double(21)").unwrap().to_string(), "42");
```

Rules:
- Names are **case-insensitive** (`DOUBLE(1)` works). Return lowercase names.
- `call` returns `None` for names you don't handle; `Some(Err(msg))` for real
  failures (surfaced as engine `MathError`).
- `init()` runs once on `register()` — validate tables, preload data, fail
  with `Err(reason)` to abort registration.
- Keep providers **pure and thread-safe** (`Send + Sync`); no I/O unless you
  declare a non-`Pure` capability and the host allows it.

## 2. Capabilities

```rust
pub enum Capability { Pure, Trigonometry, Statistics, Units, History, Graphing, Custom(String) }
```

Advertise honestly via `ModuleMetadata::with_capability(..)`. Hosts filter with
`registry.with_capability(&Capability::Pure)` — e.g. ODS-os sandboxes may
reject everything but `Pure`. A matrix solver would use
`Capability::Custom("linalg".into())`.

## 3. Unit modules

```rust
use ocalc_modules::{CalculatorModule, ModuleMetadata, UnitProvider};

pub struct Furlongs;
impl UnitProvider for Furlongs {
    fn unit_names(&self) -> Vec<String> { vec!["fur".into(), "furlong".into()] }
    fn convert(&self, value: f64, from: &str, to: &str) -> Option<Result<f64, String>> {
        // Return None when neither endpoint is yours.
        None
    }
}
```

For simple linear units you usually don't need a module at all — use
`CustomUnitRegistry::register_custom("smoot", &["smoots"], "m", 1.7018)`
from `ocalc-units`. Write a `UnitProvider` only for non-linear or
context-dependent conversions.

## 4. Error handling

- Function failures: `Some(Err("expected positive input".into()))` →
  engine `MathError` with the call-site span attached.
- Arity: check `args.len()` yourself; the engine maps unknown names to
  `UnknownFunction` only when **no** module claims them.
- Numbers: reuse `Number::{add, sub, mul, div, pow, factorial, …}` so
  `DivisionByZero`/`Overflow`/`TypeError` stay uniform.

## 5. Testing your module

```rust
#[test]
fn dispatches() {
    use ocalc_modules::ModuleRegistry;
    let mut r = ModuleRegistry::new();
    r.register(Arc::new(DoubleModule { provider: Double })).unwrap();
    assert!(r.provides_function("double"));
    assert_eq!(r.call_function("double", &[Number::Integer(21)]).unwrap(),
               Number::Integer(42));
}
```

Also test through the engine (`Calculator::eval`) to cover case-insensitivity
and span attachment, and through `sample_expression` if your function must
work in graph mode (it will — sampling clones the `Arc` registry per point).

## 6. Checklist before PR

- [ ] `id` is unique (`kebab-case`), version is semver, description is one line.
- [ ] All names lowercase; `function_names()` sorted/deduped if many.
- [ ] `init()` validates; duplicate registration yields `AlreadyRegistered`.
- [ ] No `unwrap()` on user input; all failures are `Err(String)`.
- [ ] Unit + integration tests; `cargo test`, `cargo clippy -- -D warnings` clean.
- [ ] Docs: what the function does, domain, example (`double(21) = 42`).
