# Scientific Modules (Phase 6)

Ocalculator is a **modular scientific platform**, not a single calculator app.
The `ocalc-engine` core (lexer → parser → AST → optimizer → evaluator) is
**frozen**: all Phase 6 science lives in independent crates that plug in
through the existing module system (`ocalc-modules`).

## Crates

| Crate | Id | What it provides |
|---|---|---|
| `ocalc-cas` | `cas` | Computer algebra: `SymExpr` tree, `Symbol`, `simplify`, `expand`, `factor`, `substitute`, `differentiate`, `integrate`, `solve_linear` / `solve_quadratic` |
| `ocalc-physics` | `physics` | SI constants + mechanics + electricity, via the existing unit system |
| `ocalc-chemistry` | `chemistry` | 118-element table, formula parser, molar masses |
| `ocalc-finance` | `finance` | Percent, compound interest, loans, growth |

Every crate exposes **two surfaces**:

1. **Native Rust API** — full power (symbolic trees, formula strings, structs).
2. **Engine bridge** — a `CalculatorModule` + `FunctionProvider` exposing only
   numeric kernels (engine calls carry `Number`, never trees or strings).

## Module API

```rust
use std::sync::Arc;
use ocalc_engine::CalculatorEngine;

let mut engine = CalculatorEngine::new();

// Discovery: build fresh instances (cheap, stateless providers).
let modules: Vec<Arc<dyn ocalc_engine::CalculatorModule>> = vec![
    Arc::new(ocalc_cas::CasModule::new()),
    Arc::new(ocalc_physics::PhysicsModule::new()),
    Arc::new(ocalc_chemistry::ChemistryModule::new()),
    Arc::new(ocalc_finance::FinanceModule::new()),
];
for m in modules {
    let _ = engine.register_module(m); // duplicates ignored
}

// Enable / disable at runtime (stable API only):
engine.context_mut().modules.unregister("physics"); // disable
engine.register_module(Arc::new(ocalc_physics::PhysicsModule::new())).unwrap(); // re-enable

// Function search (case-insensitive substring):
let hits: Vec<String> = engine.function_names()
    .into_iter()
    .filter(|n| n.to_ascii_lowercase().contains("force"))
    .collect();
assert_eq!(hits, vec!["force"]);
```

Traits involved (from `ocalc-modules`, re-exported by the engine):

- `CalculatorModule`: `metadata()` (id / semver / description / capabilities),
  `init()` (runs once on `register`, abort registration with `Err`),
  `as_function_provider()` / `as_unit_provider()` hooks.
- `FunctionProvider`: `function_names()` (lowercase, sorted),
  `provides(name)` (cheap membership), `call(name, args)` returning
  `None` for foreign names or `Some(Ok/Err)` for handled calls.
- `ModuleRegistry`: `register`, `register_checked`, `register_all`,
  `unregister`, `provides_function`, `call_function`, `describe`,
  `with_capability`.

Rules:

- Names are **lowercase, case-insensitive** (`FORCE(2,3)` works).
- `call` returns `None` for names you don't handle; `Some(Err(msg))` becomes
  an engine `MathError` with the call-site span.
- `init()` validates tables / runs a self-test (e.g. CAS checks
  `d/dx x^2 == 2*x`; chemistry checks `H2O == 18.015`).
- Providers are pure, `Send + Sync`, no I/O; declare non-`Pure` capabilities
  honestly (`Custom("cas")`, `Custom("physics")`, …).
- Never touch `ocalc-engine`: the evaluator only talks to `ModuleRegistry`.

## Function registration

New math arrives as a module, never as a core edit:

```rust
use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
use ocalc_number::Number;

#[derive(Debug, Clone, Default)]
pub struct MyProvider;
impl FunctionProvider for MyProvider {
    fn function_names(&self) -> Vec<String> { vec!["myfn".into()] }
    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
        if name.eq_ignore_ascii_case("myfn") && args.len() == 1 {
            Some(args[0].add(&args[0]).map_err(|e| e.to_string()))
        } else { None }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MyModule { provider: std::sync::Arc<MyProvider> }
impl CalculatorModule for MyModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata::new("my-mod", env!("CARGO_PKG_VERSION"), "one-line description")
            .with_capability(Capability::Custom("science".into()))
    }
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        Some(self.provider.as_ref())
    }
}
```

Checklist (see also `docs/MODULE_GUIDE.md`):

- `id` unique kebab-case, semver version, one-line description.
- `function_names()` lowercase + sorted; `provides()` consistent.
- `init()` self-tests; no `unwrap()` on user input.
- Unit tests (native kernels) + provider dispatch + `ModuleRegistry` loading.
- Engine end-to-end test (`CalculatorEngine::eval("myfn(21)") == 42`).
- `cargo test --workspace` + `cargo clippy --workspace --all-targets` clean.

## Examples

### CAS (`ocalc-cas`)

Native symbolic API (engine expressions evaluate to numbers and cannot carry
trees, so this stays Rust-side):

```rust
use ocalc_cas::{add, differentiate, expand, factor, konst, mul, pow, simplify, sin, substitute, var};

// x + x -> 2*x ; x*x -> x^2
assert_eq!(simplify(&add(vec![var("x"), var("x")])).to_string(), "2*x");
assert_eq!(simplify(&mul(vec![var("x"), var("x")])).to_string(), "x^2");

// (x+1)^2 -> x^2 + 2*x + 1
assert_eq!(expand(&pow(add(vec![var("x"), konst(1.0)]), konst(2.0))).to_string(), "x^2 + 2*x + 1");

// d/dx x^2 = 2*x ; integral x dx = 0.5*x^2
assert_eq!(differentiate(&pow(var("x"), konst(2.0)), "x").unwrap().to_string(), "2*x");
assert_eq!(ocalc_cas::integrate(&var("x"), "x").unwrap().to_string(), "0.5*x^2");

// x + 5 = 10 -> 5 ; x^2 - 4 = 0 -> [-2, 2]
assert_eq!(ocalc_cas::solve_linear(1.0, 5.0 - 10.0).unwrap(), 5.0);
assert_eq!(ocalc_cas::solve_quadratic(1.0, 0.0, -4.0).unwrap(), vec![-2.0, 2.0]);

// factor keeps verified forms only; substitute simplifies:
let _ = factor(&add(vec![pow(var("x"), konst(2.0)), konst(-4.0)]));
assert_eq!(substitute(&pow(var("x"), konst(2.0)), "x", &konst(3.0)), konst(9.0));
assert_eq!(sin(var("x")).to_string(), "sin(x)"); // sin/cos/log supported
```

Engine bridge (numeric kernels only):

| Function | Meaning | Example |
|---|---|---|
| `poly_eval(x, c0, c1, …)` | Horner `c0 + c1·x + …` | `poly_eval(2, 1, 2, 3) = 17` |
| `solve_linear(a, b)` | `a·x + b = 0` | `solve_linear(2, -4) = 2` |
| `quadratic_disc(a, b, c)` | `b² − 4ac` | `quadratic_disc(1, 0, -4) = 16` |
| `quadratic_root(a, b, c, k)` | k-th real root, `k = 1\|2` | `quadratic_root(1, 0, -4, 2) = 2` |

### Physics (`ocalc-physics`, all SI)

```rust
use ocalc_physics::{force, kinetic_energy, kmh_to_ms, voltage, GRAVITY};
assert_eq!(force(2.0, 3.0), 6.0);
assert_eq!(kinetic_energy(2.0, 3.0).unwrap(), 9.0);
assert!((kmh_to_ms(36.0) - 10.0).abs() < 1e-9); // via ocalc-units tables
assert_eq!(voltage(2.0, 3.0), 6.0);
assert_eq!(GRAVITY, 9.80665);
```

Engine functions (23): `acceleration avogadro boltzmann current
elementary_charge energy force gravity kinetic_energy kmh_to_ms momentum
ms_to_kmh planck potential_energy power power_ri power_vi power_vr resistance
speed_of_light velocity voltage weight`, where `energy` aliases
`kinetic_energy` and `power` aliases `power_vi`:

```
force(2, 3) = 6            energy(2, 3) = 9
velocity(100, 10) = 10     voltage(2, 3) = 6
power(6, 2) = 12           gravity() = 9.80665
```

### Chemistry (`ocalc-chemistry`)

```rust
use ocalc_chemistry::{composition, molecular_mass, parse_formula};
assert_eq!(parse_formula("H2O").unwrap()["H"], 2);
assert!((molecular_mass("H2O").unwrap() - 18.015).abs() < 1e-9);
assert!((molecular_mass("CO2").unwrap() - 44.009).abs() < 1e-9);
assert!((molecular_mass("NaCl").unwrap() - 58.44).abs() < 1e-9);
// Groups, brackets, hydrates:
assert!(parse_formula("Ca(OH)2").is_ok());
assert!(parse_formula("CuSO4·5H2O").is_ok());
let parts = composition("H2O").unwrap(); // per-element mass + percent
assert!((parts.iter().map(|c| c.percent).sum::<f64>() - 100.0).abs() < 1e-9);
```

Engine bridge (formula strings cannot cross engine calls, so numeric pairs):

| Function | Meaning | Example |
|---|---|---|
| `element_mass(z)` | atomic weight of `z` | `element_mass(8) = 15.999` |
| `molar_mass(z1, n1, z2, n2, …)` | Σ mass from `(Z, count)` pairs | `molar_mass(1, 2, 8, 1) = 18.015` (H₂O) |

### Finance (`ocalc-finance`, rates are decimals: `0.05` = 5 %)

```rust
use ocalc_finance::{compound, growth_rate, loan_payment, percent};
assert_eq!(percent(200.0, 15.0).unwrap(), 30.0);
assert!((compound(1000.0, 0.05, 1.0).unwrap() - 1050.0).abs() < 1e-9);
assert!((loan_payment(10_000.0, 0.06, 12.0).unwrap() - 860.66).abs() < 0.01);
assert!((growth_rate(100.0, 121.0, 2.0).unwrap() - 0.1).abs() < 1e-12);
```

Engine functions (9): `compound effective_rate growth growth_rate
loan_interest loan_payment loan_total percent percent_change`:

```
percent(200, 15) = 30
percent_change(100, 120) = 20
compound(1000, 0.05, 10) ≈ 1628.89
loan_payment(10000, 0.06, 12) ≈ 860.66
growth(100, 0.1, 2) = 121
```

## UI integration (no redesign)

- **Discovery**: `ocalc-desktop/src/modules.rs`
  (`discover_scientific_modules`, `create_module`, `register_scientific_modules`,
  `search_functions`) and the CLI equivalents in `ocalc-cli/src/main.rs`.
  Both register through `CalculatorEngine` / `Calculator` stable APIs.
- **Enable/disable**: desktop settings panel lists `cas / chemistry / finance /
  physics` with On/Off toggles (`AppState::set_module_enabled` unregisters or
  re-registers via the factory); CLI has `:enable <id>` / `:disable <id>` plus
  the `modules` inventory command.
- **Function search**: desktop settings panel has a search box
  (`AppState::filtered_functions`, capped at 100, case-insensitive); CLI has
  `ocalc functions --search <q>` and `:functions [query]` in the REPL.

## Testing

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

Coverage per module: native-kernel unit tests, provider dispatch tests
(incl. arity / domain errors + case-insensitivity), `ModuleRegistry` loading
tests (metadata validation + API compatibility), engine end-to-end tests
(`force(2,3)`, `compound(…)`, `molar_mass(…)`, `solve_linear(…)`), and
desktop discovery/search/enable-disable tests (`modules.rs`, `state.rs`,
`app.rs`). The engine crate itself is untouched and its suite still passes
unmodified.
