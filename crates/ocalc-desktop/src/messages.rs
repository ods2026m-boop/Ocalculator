//! [`Message`]: every user action in the desktop app.
//!
//! Flow (enforced by `app.rs`): `User action → Message → Engine API →
//! Result → UI update`. Messages carry *intent only* — never computed values.
//! All calculation happens in `commands.rs` via the stable engine API.

use crate::ods_theme::ThemeMode;
use crate::state::{AngleUnit, CalcMode, NumBase, NumberFormat, PhysicsFormula, UnitCategoryId};

/// All UI events.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Input / evaluate -------------------------------------------------
    /// Main expression input changed (typed).
    InputChanged(String),
    /// A keypad button appended a literal (`"7"`, `"+"`, `"sin("`, …).
    AppendInput(String),
    /// Evaluate the current input (`=` button, Enter key, `eval` action).
    Evaluate,
    /// Clear input + display + error (C).
    Clear,
    /// Clear the input entry only (CE).
    ClearEntry,
    /// Delete the last character.
    Backspace,
    /// Negate the current input (`±` button).
    Negate,

    // -- Memory ------------------------------------------------------------
    /// MC: clear memory. MR: recall. MS: store. MAdd/MSub: combine.
    MemoryClear,
    MemoryRecall,
    MemoryStore,
    MemoryAdd,
    MemorySub,

    // -- Modes / settings ---------------------------------------------------
    /// Switch calculator mode (tabs, `Ctrl+1…9`).
    ModeChanged(CalcMode),
    /// Trig angle unit (RAD/DEG/GRAD selector).
    AngleUnitChanged(AngleUnit),
    /// Display precision changed (decimal places 0–12).
    PrecisionChanged(u8),
    /// Result number format changed (standard / scientific).
    NumberFormatChanged(NumberFormat),
    /// Theme changed (dark/light/high-contrast).
    ThemeChanged(ThemeMode),
    /// Accent preset selected (index into accent presets).
    AccentChanged(Option<usize>),
    /// Font scale changed (accessibility).
    FontScaleChanged(f32),
    /// History capacity changed.
    HistoryCapacityChanged(usize),
    /// Toggle automatic history trimming to capacity.
    HistoryAutoTrimChanged(bool),
    /// Startup mode changed.
    StartupModeChanged(CalcMode),
    /// Toggle history sidebar visibility.
    ToggleHistory,
    /// Toggle settings panel visibility.
    ToggleSettings,
    /// Persist settings to disk.
    SettingsSave,
    /// Dismiss the current toast.
    DismissToast,

    // -- History -------------------------------------------------------------
    /// History search query changed.
    HistorySearchChanged(String),
    /// Reuse a history entry's expression (sidebar click).
    HistoryReuse(u64),
    /// Delete one history entry.
    HistoryDelete(u64),
    /// Clear all history.
    HistoryClear,
    /// Copy the current display text to the clipboard.
    CopyResult,
    /// Star/unstar the current input + display.
    FavoriteToggleCurrent,
    /// Star/unstar a history entry by id.
    FavoriteToggleEntry(u64),
    /// Reuse a favorite's expression.
    FavoriteReuse(u64),
    /// Delete one favorite.
    FavoriteDelete(u64),
    /// Clear all favorites.
    FavoritesClear,

    // -- Graph ----------------------------------------------------------------
    /// Edit function `id`'s expression.
    GraphFuncChanged(usize, String),
    /// Add a function slot.
    GraphAddFunc,
    /// Remove function `id`.
    GraphRemoveFunc(usize),
    /// Toggle function `id` visibility.
    GraphToggleFunc(usize),
    /// Graph range bound changed: (which, value). `which`: 0 = from, 1 = to.
    GraphBoundChanged(usize, String),
    /// Graph point count changed.
    GraphPointsChanged(usize),
    /// Zoom in/out.
    GraphZoomIn,
    GraphZoomOut,
    /// Pan left/right (fraction of span is fixed in state).
    GraphPanLeft,
    GraphPanRight,
    /// Reset zoom/pan.
    GraphResetView,
    /// Toggle grid.
    GraphToggleGrid,
    /// Canvas hover moved to data coordinates `(x, y)`.
    GraphHover(f64, f64),

    // -- Converter --------------------------------------------------------------
    /// Converter category changed.
    ConverterCategoryChanged(UnitCategoryId),
    /// Converter numeric input changed.
    ConverterValueChanged(String),
    /// Converter unit selection changed: (which, symbol). 0 = from, 1 = to.
    ConverterUnitChanged(usize, String),
    /// Speed time-unit changed: (which, symbol). 0 = from-time, 1 = to-time.
    ConverterTimeUnitChanged(usize, String),
    /// Swap converter from/to units.
    ConverterSwap,

    // -- Programmer ---------------------------------------------------------------
    /// Display base changed.
    ProgrammerBaseChanged(NumBase),
    /// Toggle bit-view visibility.
    ProgrammerToggleBits,
    /// Signed/unsigned display toggle.
    ProgrammerSignedChanged(bool),
    /// Toggle bit `i` of the current integer result.
    ProgrammerToggleBit(u32),

    // -- Scientific modules (Phase 6: discovery / enable / search) ------------
    /// Enable/disable a scientific module by id (toggle).
    ModuleToggled(String),
    /// Function search query changed.
    FunctionSearchChanged(String),

    // -- CAS (Phase 7: symbolic UI) -----------------------------------------
    /// CAS expression input changed.
    CasInputChanged(String),
    /// CAS variable changed.
    CasVarChanged(String),
    /// Simplify the CAS input.
    CasSimplify,
    /// Expand the CAS input.
    CasExpand,
    /// Factor the CAS input.
    CasFactor,
    /// Differentiate the CAS input.
    CasDifferentiate,
    /// Integrate the CAS input.
    CasIntegrate,
    /// Linear-solver field changed: (which, value). 0 = a, 1 = b.
    CasLinChanged(usize, String),
    /// Quadratic-solver field changed: (which, value). 0 = a, 1 = b, 2 = c.
    CasQuadChanged(usize, String),
    /// Solve `a·x + b = 0`.
    CasSolveLinear,
    /// Solve `a·x² + b·x + c = 0`.
    CasSolveQuadratic,
    /// Clear the CAS expression history.
    CasHistoryClear,
    /// Reuse a CAS history entry's input.
    CasReuse(usize),
    /// Collapse/expand the CAS solver panel.
    CasToggleSolvers,
    /// Collapse/expand the CAS history panel.
    CasToggleHistory,

    // -- Physics (Phase 7: formula UI) ---------------------------------------
    /// Physics formula selected.
    PhysicsFormulaChanged(PhysicsFormula),
    /// Physics first input changed.
    PhysicsAChanged(String),
    /// Physics second input changed.
    PhysicsBChanged(String),
    /// Collapse/expand the constants reference panel.
    PhysicsToggleConstants,

    // -- Chemistry (Phase 7: formula UI) -------------------------------------
    /// Chemistry formula input changed.
    ChemistryFormulaChanged(String),
    /// Select an element for the info panel (symbol).
    ChemistrySelect(String),
    /// Periodic-table search changed.
    ChemistrySearchChanged(String),

    // -- Finance (Phase 7: calculators) --------------------------------------
    /// Compound field changed: (which, value). 0 = principal, 1 = rate %, 2 = periods.
    FinanceCompoundChanged(usize, String),
    /// Effective-rate field changed: (which, value). 0 = nominal %, 1 = per year.
    FinanceNominalChanged(usize, String),
    /// Loan field changed: (which, value). 0 = principal, 1 = APR %, 2 = months.
    FinanceLoanChanged(usize, String),
    /// Growth-value field changed: (which, value). 0 = start, 1 = rate %, 2 = periods.
    FinanceGrowthChanged(usize, String),
    /// Growth-rate field changed: (which, value). 0 = start, 1 = end, 2 = periods.
    FinanceRateChanged(usize, String),
    /// Percent field changed: (which, value). 0 = base, 1 = percent.
    FinancePercentChanged(usize, String),
    /// Percent-change field changed: (which, value). 0 = old, 1 = new.
    FinanceChangeChanged(usize, String),

    /// Keyboard event from the subscription.
    KeyboardEvent(iced::keyboard::Event),

    /// No-op (e.g. swallowed events).
    Nop,
}
