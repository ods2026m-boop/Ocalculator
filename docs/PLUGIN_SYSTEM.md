# OCalc Plugin System (Phase 3)

New math arrives as **modules**, never as core edits. This document is the
normative reference for discovery, validation, compatibility, capabilities and
the hardened registration path. For the step-by-step tutorial see
`MODULE_GUIDE.md`; for the stable API surface see `API.md`.

## 1. Concepts

```rust
use ocalc_modules::{CalculatorModule, FunctionProvider, UnitProvider};

pub trait CalculatorModule: Send + Sync {
    fn metadata(&self) -> ModuleMetadata;                          // required
    fn name(&self) -> String;        // default: metadata().id
    fn version(&self) -> String;     // default: metadata().version
    fn capabilities(&self) -> Vec<Capability>;
    fn functions(&self) -> Vec<String>;
    fn init(&self) -> Result<(), String>;                          // default Ok
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider>; // default None
    fn as_unit_provider(&self) -> Option<&dyn UnitProvider>;         // default None
}

pub trait FunctionProvider: Send + Sync {
    fn function_names(&self) -> Vec<String>;
    fn provides(&self, name: &str) -> bool;   // default scans function_names
    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>>;
}
```

Rules (unchanged from Phase 2, enforced harder in Phase 3):

- Names are **case-insensitive**; return lowercase names.
- `call` returns `None` for foreign names (registry tries the next module),
  `Some(Err(msg))` for real failures (surfaced as engine `MathError` with the
  call-site span).
- `init()` runs once on `register()` — validate tables, preload data, fail
  with `Err(reason)` to abort registration.
- Providers must be **pure and thread-safe** (`Send + Sync`); no I/O unless a
  non-`Pure` capability is declared and the host allows it.
- `as_function_provider` must borrow owned state, never a temporary.

## 2. Metadata and validation

```rust
use ocalc_modules::{ModuleMetadata, ENGINE_PLUGIN_API_VERSION};

// Version your module in lockstep with the engine pre-1.0 (currently 0.1.x)
// so strict hosts accept it; see §3 for the policy.
let meta = ModuleMetadata::new("my-stats", ENGINE_PLUGIN_API_VERSION, "robust descriptives")
    .with_capability(Capability::Statistics);

meta.validate()?;   // id / semver / description / capabilities / format_version
```

`validate()` rejects:

| Field | Rule |
|---|---|
| `id` | 1–64 chars, kebab-case `[a-z0-9-]`; no leading/trailing/double dashes |
| `version` | strict `major.minor.patch` (+ optional `-pre` / `+build`) |
| `description` | 1–256 chars, non-blank |
| `capabilities` | non-empty, deduplicated |
| `format_version` | must equal `MODULE_METADATA_FORMAT_VERSION` (1) |

Violations yield `ModuleError::InvalidMetadata { id, reason }`. `register()`
validates **before** `init()`, so bad plugins never half-initialize.

Serialization:

```rust
let json = meta.to_json()?;                 // {"format_version":1,"metadata":{…}}
let back = ModuleMetadata::from_json(&json)?;  // validates on load
```

## 3. Compatibility

`ENGINE_PLUGIN_API_VERSION` tracks the workspace version pre-1.0 (currently
`0.1.0`; it becomes `"1.0.0"` at the first stable release). Policy (v1):

- Major versions must match.
- Module minor ≤ engine minor (a module needing a *newer* host is rejected).
- Any prerelease requires an exact version match.

```rust
meta.check_compatibility(ENGINE_PLUGIN_API_VERSION)?;  // ok when majors match

// Host side:
registry.register_checked(module, ENGINE_PLUGIN_API_VERSION)?;  // strict
registry.check_compatibility_all(ENGINE_PLUGIN_API_VERSION);  // (compatible_ids, errors)
engine.register_module_checked(module, ENGINE_PLUGIN_API_VERSION)?;
```

`register()` checks metadata but not compatibility (lenient, for tests and
trusted first-party modules). Production frontends, the CLI `--strict` shape
and ODS-os sandboxes must use `register_checked`.

`ModuleError` variants: `AlreadyRegistered | NotFound | InitFailed |
Provider | InvalidMetadata | IncompatibleVersion`.

## 4. Discovery

Rust has no runtime dynamic loading in this workspace, so *discovery* means
composing inventory vectors and bulk-registering them with aggregated errors:

```rust
use std::sync::Arc;
use ocalc_engine::{CalculatorEngine, CalculatorModule};

// 1. Build the inventory (built-ins + third-party crates):
let inventory: Vec<Arc<dyn CalculatorModule>> = vec![
    Arc::new(ocalc_functions::StandardMathModule::new()),
    Arc::new(ocalc_functions::ScientificModule::new()),
    Arc::new(ocalc_functions::StatisticsModule::new()),
    Arc::new(ocalc_units::UnitsModule::new()),
    Arc::new(my_crate::MyModule::new()),
];

// 2a. Strict host (recommended): validate + compat, collect every failure:
let mut engine = CalculatorEngine::new();
let mut good = Vec::new();
let mut errors = Vec::new();
for m in inventory {
    match m.metadata().validate().and_then(|_| m.metadata().check_compatibility(ENGINE_PLUGIN_API_VERSION)) {
        Ok(()) => match engine.register_module(m) {
            Ok(()) => good.push(()),
            Err(e) => errors.push(e.to_string()),
        },
        Err(e) => errors.push(e.to_string()),
    }
}

// 2b. Or bulk helper (validates + inits, registers successes, returns failures):
let mut registry = ocalc_modules::ModuleRegistry::new();
if let Err(errs) = registry.register_all(inventory) {
    for e in errs { eprintln!("plugin rejected: {e}"); }
}

// 3. Inspect:
registry.module_ids();
registry.describe();   // (name, version, functions, capabilities)
registry.validate_all();
```

`CalculatorEngine::new()` pre-registers the four built-ins
(`standard-math`, `scientific-math`, `statistics`, `units`); `register_builtin_modules`
is idempotent (duplicate ids skipped). `register_all` never aborts early: good
modules land even when siblings fail.

## 5. Capabilities

```rust
pub enum Capability { Pure, Trigonometry, Statistics, Units, History, Graphing, Custom(String) }
```

Advertise honestly. Hosts filter with `with_capability`:

```rust
registry.with_capability(&Capability::Pure);            // sandbox-safe set
registry.with_capability(&Capability::Trigonometry);    // ["scientific-math"]
```

Built-ins: `standard-math → [Pure]`, `scientific-math → [Pure, Trigonometry]`,
`statistics → [Pure, Statistics]`, `units → [Pure, Units]` (see
`ocalc-units/src/module.rs`). A matrix solver would declare
`Custom("linalg")`. ODS-os sandboxes allow only `Pure`.

`Capability` is JSON-serializable (`Custom("x")` → `{"Custom":"x"}`).

## 6. Dispatch performance

- Built-ins: single `FunctionRegistry::call` (no prior `contains` probe).
- Modules: single `ModuleRegistry::call_function` scan (no prior
  `provides_function` probe in the evaluator).
- Override `FunctionProvider::provides` / `UnitProvider::provides` when
  `function_names()` / `unit_names()` would allocate large vectors per call;
  the default scans the vec case-insensitively.
- Keep `call` allocation-free on the miss path (compare lowercased names
  without formatting).

## 7. Testing your module

```rust
#[test]
fn dispatches() {
    let mut r = ModuleRegistry::new();
    r.register(Arc::new(MyModule::new())).unwrap();
    assert!(r.provides_function("myfn"));
    assert_eq!(r.call_function("myfn", &[Number::Integer(21)]).unwrap(),
               Number::Integer(42));
}

#[test]
fn metadata_is_valid_and_compatible() {
    let m = MyModule::new();
    let meta = m.metadata();
    meta.validate().unwrap();
    meta.check_compatibility(ENGINE_PLUGIN_API_VERSION).unwrap();
}

#[test]
fn through_the_engine() {
    let mut e = CalculatorEngine::new();
    e.register_module(Arc::new(MyModule::new())).unwrap();
    assert_eq!(e.eval("MYFN(21)").unwrap().to_string(), "42");  // case-insensitive
}
```

Also exercise `sample_graph` when the function must work in graph mode
(sampling clones the `Arc` registry per point), and `to_json`/`from_json` for
the metadata. Run `cargo test`, `cargo clippy -- -D warnings`, and the
`tests/security.rs` adversarial corpus against any parser-facing surface.

## 8. Checklist before PR

- [ ] `id` kebab-case unique, `version` semver, `description` one line ≤256.
- [ ] Names lowercase; `provides` overridden when the inventory is large.
- [ ] `init()` validates tables/data; duplicate registration → `AlreadyRegistered`.
- [ ] No `unwrap()` on user input; failures are `Err(String)`.
- [ ] `validate()` + `check_compatibility()` green; `register_checked` green.
- [ ] Unit + engine-integration tests; fuzz the name parser if it parses text.
- [ ] Docs: purpose, domain, example (`myfn(21) = 42`), capability rationale.
