# Ocalculator UX Guide (Phase 8)

Professional desktop UX polish for the Iced frontend. The engine
(`ocalc-engine`) and scientific crates (`ocalc-cas`, `ocalc-physics`,
`ocalc-chemistry`, `ocalc-finance`) are **untouched** — every number on
screen flows through `ocalc-desktop/src/commands.rs` via stable public APIs.
This guide covers the presentation layer only: structure, navigation,
themes, shortcuts and component rules.

Run it: `cargo run -p ocalc-desktop`

## 1. UI structure

Iced architecture is preserved everywhere: `State -> Message -> Update ->
View`.

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
| `main.rs` | Iced entry (`application`, window size 1100×720) | runtime wiring only |
| `app.rs` | `update`/`view`/`subscription`/`theme`, keyboard map, settings autosave, focus tasks | `AppState` + `Message` |
| `messages.rs` | `Message` enum (input, memory, modes, settings, history, graph, converter, programmer, CAS, physics, chemistry, finance, keyboard) | intent payloads |
| `state.rs` | `AppState` + tab states + `Settings`; all transitions | `commands::*` |
| `commands.rs` | **only** engine calls: eval/format, grad rewrite, speed composition, memory ops, history rebuild, CAS/physics/chemistry/finance bridges | `ocalc_*` APIs |
| `views/shell.rs` | header, toolbar, content row, toast, status bar | state reads |
| `views/*.rs` | one module per mode/panel, shared primitives in `views/mod.rs` | state reads |
| `ods_theme/` | colors, spacing, typography, button styles | tokens only |

Rules (unchanged from Phase 5):

- Views never compute: no arithmetic, parsing, or unit math outside
  `commands.rs` (formatting an existing `Value` or timestamp for display is
  allowed).
- `Message`s carry literals (`AppendInput("sin(")`), selections, and ids —
  never results.
- Filesystem access (settings load/save) happens in `OcalcApp::new`/`update`
  only; `update_inner` stays pure for tests.

### Window layout (`views/shell.rs`)

```text
Main Window (resizable, Fill everywhere, md outer padding)
├── Header: "Ocalculator" + mode subtitle + memory indicator (● M / ○ M) + History/Settings toggles
├── Toolbar row 1: Standard (1) … Graph (5) + CE / C quick actions
├── Toolbar row 2: "Scientific:" CAS (6) Physics (7) Chemistry (8) Finance (9)
├── Content row (height Fill): calculator area (Fill) + history (300px) + settings (340px)
│   └── sidebars collapse via header toggles → narrow windows stay usable
├── Toast (⚠ error / ℹ info + Dismiss)
└── Status bar (muted, tiny): mode • angle • precision • format • history N • ★ N • evals N (last µs)
```

Responsive resizing:

- The calculator area is `Length::Fill` in both directions; sidebars are
  fixed width only. The content row is `height: Fill` so the status bar stays
  pinned and the mode view reflows instead of clipping.
- Long panels scroll: history list, CAS history, chemistry browser (160px),
  finance body, settings panel (full height). Graph canvas is fixed 320px
  tall (stable rendering) but full width.
- Settings groups stack vertically (label above controls) so the 340px panel
  never overflows.

## 2. Navigation

Nine modes, tab order = shortcut order:

| Shortcut | Mode | Primary input (autofocus) |
|---|---|---|
| Ctrl+1 / Cmd+1 | Standard | `calc-input` |
| Ctrl+2 / Cmd+2 | Scientific | `calc-input` |
| Ctrl+3 / Cmd+3 | Programmer | — (keypad-first) |
| Ctrl+4 / Cmd+4 | Converter | `conv-value` |
| Ctrl+5 / Cmd+5 | Graph | — (canvas-first) |
| Ctrl+6 / Cmd+6 | CAS | `cas-input` |
| Ctrl+7 / Cmd+7 | Physics | `physics-a` |
| Ctrl+8 / Cmd+8 | Chemistry | `chem-formula` |
| Ctrl+9 / Cmd+9 | Finance | `fin-compound-p` |

- The toolbar highlights the active mode with the accent `primary_button`
  style; inactive modes stay quiet (`subtle_button`). Labels always show the
  digit hint, e.g. `Standard (1)`.
- `Message::ModeChanged` moves keyboard focus to the mode's primary input
  (`focus_task_for`); Graph re-samples on entry.
- Shortcuts accept both `Ctrl` (Linux/Windows) and `Cmd` (macOS). `Ctrl`/`Cmd`
  with any other key is swallowed — never appended to the expression.
- Printable calculator keys (`0–9 + - * / % ^ ( ) . !`) append when no text
  input consumed them; `=` evaluates. `Enter` evaluates, `Esc` clears all,
  `Backspace` deletes one char, `Delete` clears the entry (keeps the result).

## 3. Themes (ODS)

`ods_theme/` is the single source of truth. Views read tokens every frame;
nothing hardcodes sizes or colors except the defaults here.

```text
ods_theme/
├── colors.rs     — ThemeMode (Dark/Light/HighContrast→Oxocarbon), OdsColors, ACCENT_PRESETS
├── spacing.rs    — Spacing scale (xs 4 / sm 8 / md 12 / lg 20, logical px)
├── typography.rs — Typography (display 32 / heading 20 / body 16 / small 13 / tiny 12) + scaled()
└── components.rs — primary_button / danger_button / subtle_button (Active/Hovered/Pressed/Disabled)
```

- `OdsTheme { mode, colors, spacing, typography, font_scale }`; every view
  reads `scaled_typography()` per frame — font scaling (87.5–150%) is a
  one-line change everywhere.
- Dark is the default; Light retunes accent/success/danger/muted;
  High contrast maps to Iced `Oxocarbon` with pure accents.
- Accent presets (blue/green/violet/orange/red) override `colors.accent`;
  `Auto` restores the per-mode default. The active theme/accent/format/angle
  control uses the accent style; inactive controls stay subtle.
- Status, hints, timestamps and empty states always use `muted`; errors always
  use `danger` with a `⚠` prefix plus a one-line correction hint
  (`Check parentheses…`, `Adjust the inputs…`).

### Spacing / typography discipline (Phase 8)

- Gaps: `sp.xs` inside keypads/rows, `sp.sm` between sections, `sp.md` for
  outer padding and card padding. No bare `4`/`6`/`8`/`12` in new code.
- Type: `display` for results only, `heading` for panel titles, `body` for
  inputs and primary labels, `small` for controls, `tiny` for hints, status,
  timestamps and dense rows. The header title is `heading`; the mode subtitle
  and memory indicator are `small`.
- Cards: `section_card(title, content)` for physics inputs, finance
  calculators, CAS-adjacent groups. Card = title (`body`) + content with
  `xs` spacing, `md` padding, full width.

## 4. Shortcuts reference

The settings panel ends with the full reference (`views::settings::SHORTCUTS`,
17 rows). Keep it in sync with `CalcMode::ALL` — there is a test
(`phase8_shortcut_reference_covers_all_modes`) that fails if a mode lacks a
`Ctrl+N` row.

| Keys | Action |
|---|---|
| Enter / = | Evaluate expression |
| Esc | Clear input + result |
| Backspace | Delete last character |
| Delete | Clear entry (keep result) |
| 0–9 + - * / % ^ ( ) . ! | Append to input |
| Ctrl+1 … Ctrl+9 (or Cmd) | Switch modes (all nine) |
| Tab / Shift+Tab | Move keyboard focus (native) |
| Click history entry | Reuse expression |
| ☆ / ★ | Star / unstar favorite |

## 5. Component guidelines

Shared primitives live in `views/mod.rs`. Use them; do not invent new
styles.

- `key(label, msg)` — quiet keypad button (`subtle_button`, fill width).
- `accent_key(label, msg, accent)` — evaluate/apply (`primary_button`).
- `display_panel(input, display, error, danger, ty, sp)` — expression
  `text_input` (`calc-input` id, `Enter` submits) + result (`display` size)
  or muted `Type an expression…` placeholder, plus `⚠ <first line>` and a
  correction hint on error.
- `result_panel(value, error, empty, ty, danger, muted)` — `= value`
  (`display×0.875`), or `⚠ <first line>` + `Adjust the inputs…`, or centered
  muted empty text. Used by CAS, physics, chemistry.
- `section_card(title, content, ty, sp)` — titled input group (physics,
  finance).
- `labeled_field(label, value, id, on_input, small)` — 140px label + fill
  input with 6px padding; pass the focus `id` only for the primary field per
  mode (`physics-a`, `fin-compound-p`, …).
- `heading(label, size)` / `hint(label, size)` — owned (`impl Into<String>`,
  returns `'static`), so temporaries like `format!(…)` are fine.
- `error_text(msg, size, danger)` — full-width danger line.
- `empty_state(label, size, muted)` — centered, full-width, muted. Use for
  every empty list: history (`No history yet…`), favorites (`No favorites
  yet…`), CAS history, chemistry breakdown/browser, hidden-graph plot,
  programmer bits (`Evaluate an integer first…`), function search
  (`No matching functions.`).
- Buttons: `primary_button(accent)` for the single primary action per group
  (evaluate, save, active mode), `danger_button(danger)` for destructive
  (clear history, delete entry, remove function), `subtle_button()` for
  everything else. Disabled state is automatic (no `on_press`).

### Mode notes

- **Standard**: memory row (MC/MR/MS/M+/M−, `small`), display, copy/star
  actions, 5-row keypad, keyboard hint
  (`Enter evaluates • Esc clears • ←/→ move cursor • Ctrl+1…9 switch modes`).
  Cursor navigation is native to the focused `text_input` (arrow keys,
  Home/End).
- **History sidebar**: header with count + danger Clear, search box, favorites
  shelf (`★ Favorites (N)` + Clear + `✕` danger deletes), log with
  `#id · HH:MM = result`, `☆/★` toggle and `✕` delete per entry. Newest 100
  (search) shown, newest 50 rendered with a `Showing newest 50 of N…` hint.
  Timestamps come from `HistoryEntry.timestamp` (`%H:%M`); ids stay monotonic
  via `rebuild_history`.
- **Graph**: function list panel (`Functions (n/max)` + color dot `●/○` +
  expr box + Show/Hide + `✕` danger + per-function `⚠` error + Add), range
  boxes, points presets (101/501/1001), zoom (`− Zoom`/`+ Zoom` + live
  `Zoom 1×/2.0×/1/2×`), pan, Reset, grid (`☑/☐ Grid`), canvas, status
  (`range […] • total (valid) • cursor (x, y) / hover hint`). Hidden functions
  keep their slot but clear samples; all-hidden shows an empty state instead
  of a blank canvas. Sampling stays in `commands::sample_graph_points`.
- **Settings**: five groups — Appearance (theme, accent, font size),
  Calculation (angle + hint, precision stepper, number format, exactness
  note), History (maximum slider, auto-cleanup checkbox + explanation),
  Startup & layout (startup picker, sidebar checkboxes, accent Save),
  Modules (On/Off per scientific module + descriptions + function search),
  Shortcuts (17-row reference). Scrollable, 340px, autosaves on every
  `affects_settings` message.
- **CAS**: expression editor + variable row + Simplify/Expand/Factor/d/dx/∫dx
  + result panel + collapsible Solve (`a·x+b=0`, `a·x²+b·x+c=0`) and History
  (`op: input = output` + Use + Clear + empty state).
- **Physics**: formula picker, `section_card("Inputs")` with unit-labeled
  fields (`mass (kg)`, …), result with SI unit, collapsible Constants (SI)
  reference from `commands::physics_constants`.
- **Chemistry**: formula box, molar-mass result, Composition mini-cards
  (symbol button + `name ×count (#n)` + `mass g/mol · pct %`), Element card,
  periodic-table browser (search + 10-col grid, selected as `[Sym]`, 160px
  scroll, empty `No elements match…` state).
- **Finance**: four `section_card`s (Compound, Loan, Growth, Percentages) with
  `outcome` lines (`= …`, `⚠ …` danger, `—` muted). Loan summarizes
  `= pmt /mo · total · interest`; percents show `% effective` / `%/period`.
  Rates are human percentages (`5` = 5 %); conversion to fractions lives in
  `commands`, never in the view.
- **Converter / Scientific / Programmer**: category/base/angle selectors with
  `[active]` markers, value boxes, pickers with `⇄` swap, readouts via
  `format_bases`, interactive low-16 bit view with empty
  (`Evaluate an integer first…`) state. Converter and finance errors use the
  same `⚠` + hint pattern as results.

## 6. Testing

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

Desktop suite (`cargo test -p ocalc-desktop`, 70 tests): engine-command tests
(eval, precision, grad, speed, memory, history rebuild, CAS/physics/chemistry/
finance bridges), state transitions (submit, memory, capacity, multi-graph,
converter, bits, settings roundtrip, module enable/disable, favorites,
hover), message tests (all panels, keyboard, shortcuts), theme tests, plus
Phase 8 additions:

- `phase8_navigation_all_modes_via_message` — all nine modes reachable,
  counts, unique digits, graph resample.
- `phase8_shortcuts_ctrl_and_cmd_both_switch` — `Ctrl+1…9` and `Cmd+1…9`
  both switch; `Ctrl+q` is swallowed.
- `phase8_shortcut_reference_covers_all_modes` — reference panel in sync.
- `phase8_settings_persistence_full_roundtrip` — precision/angle/theme/
  accent/font/capacity/startup/format/auto-trim/favorites survive
  save → load → `with_settings`.
- `phase8_settings_messages_wire_all_categories` — every settings category
  has a working message (appearance, calculation incl. number format,
  history incl. auto-trim, layout).
- `phase8_history_empty_search_and_favorites_flow` — empty states, search
  filter, star/reuse/unstar/clear.
- `phase8_graph_hover_visibility_and_view_controls` — hover set/cleared,
  grid preserves hover, show/hide clears/restores samples, pan/reset range.
- `phase8_copy_and_clear_actions_notify` — copy empty → error toast, copy
  result → info toast, clear + dismiss.

Keep all existing functionality working: no engine changes, no scientific
logic changes, no architecture rewrites — polish only.
