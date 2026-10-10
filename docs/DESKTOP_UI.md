# Ocalculator Desktop UI (Phase 5)

Production-quality Iced frontend powered **only** by public OCalc Engine APIs.
The engine is never modified here and no calculation logic is duplicated in
the UI: every number on screen flows through `commands.rs` → stable engine
calls (`CalculatorEngine`, `Value`, `builtin_converter`, graph sampling).

Run it: `cargo run -p ocalc-desktop`

## 1. UI architecture

```text
User action
  ↓  Message (intent only, messages.rs)
OcalcApp::update_inner (app.rs, pure + headless-tested)
  ↓  AppState method (state.rs)
commands.rs ── sole engine caller ──→ ocalc_* public APIs
  ↓  Result
AppState updated → views re-render
```

| File | Role | May touch |
|---|---|---|
| `main.rs` | Iced entry (`application`, window size) | runtime wiring only |
| `app.rs` | `update`/`view`/`subscription`/`theme`, keyboard map, settings autosave | `AppState` + `Message` |
| `messages.rs` | `Message` enum (input, memory, modes, settings, history, graph, converter, programmer, keyboard) | intent payloads |
| `state.rs` | `AppState` + tab states + `Settings`; all transitions | `commands::*` |
| `commands.rs` | **only** engine calls: eval/format, grad rewrite, speed composition, memory ops, history rebuild | `ocalc_*` APIs |
| `views/shell.rs` | header, toolbar, content row, toast, status bar | state reads |
| `views/*.rs` | one module per mode/panel, shared primitives in `views/mod.rs` | state reads |
| `ods_theme/` | colors, spacing, typography, button styles | tokens only |

Rules:

- Views never compute: no arithmetic, parsing, or unit math outside
  `commands.rs` (formatting an existing `Value` for display is allowed).
- `Message`s carry literals (`AppendInput("sin(")`), selections, and ids —
  never results.
- Filesystem access (settings load/save) happens in `OcalcApp::new`/`update`
  only; `update_inner` stays pure for tests.

## 2. Layout

```text
Main Window (resizable, Length::Fill everywhere)
├── Header: title, memory (M) indicator, history/settings toggles
├── Toolbar: mode tabs with Ctrl+1…5 hints, CE / C quick actions
├── Content row: calculator area (Fill) + history sidebar (300px) + settings (320px)
│   └── sidebars collapse via header toggles → narrow windows stay usable
├── Toast (transient info/error + Dismiss)
└── Status bar: mode • angle • precision • history count • evals (last µs)
```

Window resizing is native Iced behavior; layouts use `Fill` proportions and
fixed-width sidebars only, so the app reflows instead of clipping.

## 3. Modes

- **Standard**: display, memory row (MC/MR/MS/M+/M−), digit/operator keypad,
  CE/C, percent, factorial, ANS-style negate.
- **Scientific**: 16-function panel (sin…tanh, log/ln, sqrt/cbrt, exp, pow,
  abs), constants panel (π e τ φ ∞ NaN), RAD/DEG/GRAD selector.
- **Programmer**: base switcher (BIN/OCT/DEC/HEX, selected base leads),
  DEC/HEX/OCT/BIN readouts via `format_bases`, signed/unsigned 64-bit line,
  interactive low-16-bit view (toggles rebuild a `0b…` literal evaluated by
  the engine), shifts and bitwise keypad.
- **Converter**: 7 categories (Length, Mass, Temperature, Time, Data size,
  Energy, Speed). Categories enumerate engine symbols via public
  `lookup`/`category`; Speed composes length/time conversions with
  engine-evaluated arithmetic (the engine has no speed units).
- **Graph**: up to 4 functions (add/remove/show-hide, per-function errors),
  range controls, point presets, zoom (×1.25 steps), pan (±10% span), grid
  toggle, canvas with axes + per-series colors; discontinuities break
  polylines.

## 4. Angle units (incl. GRAD without engine changes)

The engine supports radians/degrees. The UI adds gradians as a
**boundary transform** in `commands::grad_to_radian_expr`:

- `sin|cos|tan(E)` → `sin|cos|tan((E)*pi/200)` (argument scaled, recursive)
- `asin|acos|atan(E)` → `((asin|acos|atan(E))*200/pi)` (result scaled)
- everything else (incl. `atan2`, hyperbolics, unknown names) passes through.

Only balanced-paren matching + name lookup happen in the UI; `pi` and all
trig stay in the engine. History records the user's original text. Covered by
dedicated unit tests (nesting, mixed case, passthrough).

## 5. History, memory, settings

- **History**: sidebar with search, newest-first filter, click-to-reuse,
  per-item delete (✕), clear-all. Backed solely by the public history API;
  delete/capacity rebuild via `to_json` → filter → `from_json`, so ids and
  `next_id` survive. Capacity (10–10 000) enforced on submit and on change.
- **Memory**: `Option<Value>` register; M+ / M− combine through public
  `Value::add`/`sub`; MR inserts the formatted value; header shows `M`.
- **Settings** (`~/.config/ocalc-desktop/settings.json`, best-effort):
  theme, accent preset, font scale, precision, angle unit, history capacity,
  startup mode, sidebar visibility. Loaded at boot, autosaved on
  settings-affecting messages, clamped on load. `SettingsSave` reports
  success/failure via toast.

## 6. Theme system (`ods_theme/`)

```text
ods_theme/
├── colors.rs     — ThemeMode (Dark/Light/HighContrast→Oxocarbon), OdsColors, ACCENT_PRESETS
├── spacing.rs    — Spacing scale (f32 logical pixels)
├── typography.rs — Typography scale + scaled()
└── components.rs — primary_button / danger_button / subtle_button
                    (explicit Active/Hovered/Pressed/Disabled states)
```

- `OdsTheme { mode, colors, spacing, typography, font_scale }`; every view
  reads `scaled_typography()` per frame — font scaling (87.5–150%) is a
  one-line change everywhere.
- High contrast = Oxocarbon base + pure accents; keyboard focus is visible
  through bordered hover/press states on all buttons.
- `theme/` keeps thin re-export shims (`OdsTheme`, `ThemeMode`,
  `Typography`) for existing import paths.

## 7. Keyboard support

Global subscription (printable keys stay with the focused input to avoid
doubles): `Enter`/`=` evaluate, `Esc` clear, digits/operators append,
`Ctrl/Cmd+1…5` switch modes. Full table in the settings panel
(`views::settings::SHORTCUTS`). Tab/Shift+Tab move focus natively.

## 8. Adding a new mode

1. Add a variant to `CalcMode` (+ `label`, `shortcut_digit`, `ALL`).
2. Add tab state to `AppState` (or reuse shared input) with pure methods.
3. Add `Message`s; handle them in `OcalcApp::update_inner`.
4. Add `views/<mode>.rs` with `pub fn view(state: &AppState)` reading state
   only; register it in `views/mod.rs` + the `shell.rs` match.
5. Route any new math through `commands.rs` (engine APIs only).
6. Add state/message transition tests; document the mode here.

## 9. Testing

`cargo test -p ocalc-desktop` (40 tests): engine-command tests (eval,
precision, grad, speed, memory, history rebuild), state transitions (submit,
memory flow, capacity, multi-graph, converter categories, bit toggles,
settings roundtrip), message tests (all panels, shortcuts, keyboard),
theme tests. Workspace: `cargo test --workspace`, `cargo clippy --workspace
--all-targets` — both must be clean.
