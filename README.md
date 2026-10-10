# Ocalculator — v1.0.0

A professional **modular scientific calculator** in Rust: a shared calculation
engine powers both the `ocalc` CLI and the Iced desktop app (`ocalc-desktop`),
with ODS-os integration ready.

> Engine computes, frontends render. No calculation logic lives in the UI:
> both `ocalc-cli` and `ocalc-desktop` call the same stable API
> (`CalculatorEngine`, `Expression`, `Value`, `ModuleRegistry`,
> `sample_graph`, `EngineConfig`).

## Features

- **9 desktop modes**: Standard, Scientific, Programmer (BIN/OCT/DEC/HEX +
  bit view), Converter (length/mass/temperature/time/data/energy + composite
  speed), Graph (multi-function, zoom/pan/grid/hover), CAS, Physics,
  Chemistry, Finance.
- **Engine**: lexer → tokens → parser → AST → optimize → evaluator; exact
  integers (`i64`/`BigInt`), `Rational` (`1/3` stays exact), `Decimal`,
  `Float`, `Complex`; `0x`/`0o`/`0b` literals, `_` separators, `π τ φ`
  aliases, case-insensitive `AND OR XOR NOT MOD`, operators
  `+ - * / % ^(**) ! & | ~ << >> ( ) , =`.
- **Functions**: `sin cos tan asin acos atan sinh cosh tanh atan2 log ln log2
  log10 exp sqrt cbrt pow hypot abs floor ceil round trunc fract sign min max
  clamp sum mean avg median variance stddev fact gcd lcm`, constants
  `pi e tau phi sqrt2 sqrt1_2 ln2 ln10 log2e log10e inf nan`.
- **Units**: length/mass/temperature (affine)/data/energy/time + custom
  registry + composite speed.
- **CAS**: symbolic simplify/expand/factor, differentiate/integrate, linear +
  quadratic solving.
- **Physics**: `force weight velocity acceleration momentum kinetic_energy
  potential_energy voltage current resistance power_vi power_ri power_vr` (SI).
- **Chemistry**: periodic table (118 elements), formula parser
  (`H2O`, `Ca(OH)2`), molar masses + per-element breakdown; engine kernels
  `element_mass(z)` + `molar_mass(z1,n1,…)`.
- **Finance**: `percent compound loan_payment effective_rate growth_value
  growth_rate percent_change`.
- **History**: bounded store (10 000 CLI / configurable desktop), search,
  versioned JSON (`{"format_version":1,…}`), atomic saves, legacy migration.
- **Settings**: versioned (`config_version: 1`), precision 0–12, RAD/DEG/GRAD
  (grads rewritten to exact `pi/200` factors at the UI boundary), light/dark
  + accent presets, font scale, history capacity, startup mode, number format
  (standard/scientific), favorites (starred calculations).
- **Plugin API**: `CalculatorModule` / `FunctionProvider` / `UnitProvider`,
  `ModuleMetadata` validation (kebab-case id, semver, description,
  capabilities), `register_checked` compatibility (major must match, module
  minor ≤ engine minor), `verify_signature` hook for future external/signed/
  ODS-repository modules. All built-ins (`cas`, `physics`, `chemistry`,
  `finance`) pass strict checks.
- **XDG layout** (no hardcoded `~/.config/ocalc-desktop`):
  - Config: `$XDG_CONFIG_HOME/ocalculator/` (fallback
    `~/.config/ocalculator/`) → `settings.json`
  - Data: `$XDG_DATA_HOME/ocalculator/` (fallback
    `~/.local/share/ocalculator/`) → `history.json`, `logs/ocalculator.log`
  - Cache: `$XDG_CACHE_HOME/ocalculator/` (fallback `~/.cache/ocalculator/`)
  - Legacy `~/.config/ocalc-desktop/settings.json` and
    `~/.local/share/ocalc/history.json` are migrated non-destructively.
- **Logging**: startup banner (version + resolved dirs), module load/errors,
  history I/O warnings, settings autosave failures, fatal UI errors — all to
  `~/.local/share/ocalculator/logs/ocalculator.log` plus stderr. Never silent,
  never panics on logging.
- **Limits**: expression length, token count, recursion/eval depth, arity,
  graph points, variable count — with a `sandbox()` preset for untrusted input.

## Installation

Copy the `usr/` tree onto the target root. Then:

```bash
ocalc-desktop          # desktop app
ocalc eval "sqrt(25) * 10"
ocalc                # REPL
```

Desktop launcher, icon, and AppStream metadata are included.

### From crates (local)

```bash
cargo install --path crates/ocalc-cli      # provides `ocalc`
cargo install --path crates/ocalc-desktop  # provides `ocalc-desktop`
```

## Building from source

Requirements: Rust stable 1.75+.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
cargo build --release
./target/release/ocalc eval "6 AND 3" --angle radians
./target/release/ocalc-desktop
```

Release profile: `opt-level=3`, `lto=true`, `codegen-units=1`,
`panic="abort"`, `strip=true`. Debug builds use the default dev profile and
keep working without changes.

## Usage

### CLI

```bash
ocalc eval "sin(45) + sqrt(25) * 10" --angle degrees
ocalc graph "sin(x)" --from 0 --to 6.28 --points 101 --format csv
ocalc units 1 km m
ocalc functions --search sin
ocalc modules
ocalc history show --limit 20
ocalc history search "sin"
echo "1+2" | ocalc
ocalc --history-file /tmp/h.json eval "1+1"
```

REPL: `:help :history [N] :search <q> :vars :forget <n> :degrees :radians
:functions [q] :modules :enable <id> :disable <id> :units
:graph <expr> <from> <to> [points] :clear [:clear history] :quit`.

History persists to `~/.local/share/ocalculator/history.json`
(override `--history-file`).

### Desktop

```bash
cargo run -p ocalc-desktop
```

Expression input → `Evaluate` (Enter/`=`), history sidebar (search, reuse,
delete, clear), favorites (☆/★), memory (M+/M-/MR/MC), precision selector,
RAD/DEG/GRAD, standard/scientific number format, theme (light/dark + accents +
font scale), converter, graph (up to 4 functions), CAS/physics/chemistry/
finance panels, settings (autosaved to
`~/.config/ocalculator/settings.json`).

## Keyboard shortcuts

| Keys | Action |
|---|---|
| Enter / `=` | Evaluate expression |
| Esc | Clear input + result |
| Backspace | Delete last character |
| Delete | Clear entry (keep result) |
| `0–9 + - * / % ^ ( ) . !` | Append to input |
| Ctrl+1 … Ctrl+9 (Cmd on macOS) | Standard / Scientific / Programmer / Converter / Graph / CAS / Physics / Chemistry / Finance |
| Tab / Shift+Tab | Move keyboard focus |
| Click history entry | Reuse expression |
| ☆ / ★ | Star / unstar favorite |

(Full list in the Settings panel → Shortcuts.)

## Modules

| Id | Description | Example functions |
|---|---|---|
| `cas` | Computer algebra: simplify, calculus, solving | `solve_linear`, `solve_quadratic` (native: `simplify`, `differentiate`, `factor`, `expand`) |
| `physics` | Constants, mechanics, electricity (SI) | `force`, `velocity`, `power_vi` |
| `chemistry` | Periodic table, formula parser, molar masses | `element_mass`, `molar_mass` (native: `parse_formula`, `molecular_mass`) |
| `finance` | Percent, compound, loans, growth | `percent`, `compound`, `loan_payment` |

Built-ins (`standard-math`, `scientific-math`, `statistics`, `units`) are always
available. Discovery is identical in CLI and desktop
(`register_module_checked` + `ENGINE_PLUGIN_API_VERSION`); `:enable`/`:disable`
(CLI) and the settings Modules panel (desktop) toggle the same four ids.
Third-party crates implement `CalculatorModule` without touching the engine
(see `docs/MODULE_GUIDE.md`, `docs/PLUGIN_SYSTEM.md`). External/signed/
ODS-repository loading is reserved via `ModuleSource::{External,Repository}`
+ `verify_signature` (built-ins are always trusted).


## Workspace

```text
Ocalculator/
├── Cargo.toml                  # workspace + shared deps + release profile
├── crates/
│   ├── ocalc-number/           # Number: Integer/BigInt/Rational/Decimal/Float/Complex
│   ├── ocalc-functions/        # standard/scientific/statistics + AngleMode + constants
│   ├── ocalc-modules/          # CalculatorModule / FunctionProvider / UnitProvider / registry
│   ├── ocalc-units/            # Length/Mass/Temperature/Data/Energy/Time + units module
│   ├── ocalc-matrix/           # Matrix/Vector foundation
│   ├── ocalc-solver/           # Polynomial, bisection/Newton, A·x=b
│   ├── ocalc-history/          # HistoryStore + XDG paths + migration + file logging
│   ├── ocalc-engine/           # Lexer → Parser → AST → optimize → Evaluator + graph + CalculatorEngine
│   ├── ocalc-cas/              # Symbolic CAS module
│   ├── ocalc-physics/          # Physics module
│   ├── ocalc-chemistry/        # Chemistry module
│   ├── ocalc-finance/          # Finance module
│   ├── ocalc-cli/              # `ocalc` binary (stable engine API only)
│   └── ocalc-desktop/          # Iced desktop app (stable engine API only)
└── docs/
    ├── ENGINE.md, ARCHITECTURE.md, MODULE_GUIDE.md, PLUGIN_SYSTEM.md, …
    └── RELEASE.md              # release process + package creation + testing checklist
```

Design constraints (preserved for v1.0.0): no architecture rewrite, no crate
merges, no logic in UI, Engine/Modules/Iced UI stay separated.

## Docs

- `docs/ENGINE.md` — pipeline, precision, extension.
- `docs/ARCHITECTURE.md` — crate map, data flow.
- `docs/MODULE_GUIDE.md` — write a plugin without touching the core.
- `docs/PLUGIN_SYSTEM.md` — registry, capabilities, versioning.
- `docs/SCIENTIFIC_MODULES.md` — CAS/physics/chemistry/finance reference.
- `docs/RELEASE.md` — release process, package creation, testing checklist.
