//! [`AppState`]: all mutable UI state + pure transitions.
//!
//! `AppState` owns the [`CalculatorEngine`] and [`HistoryStore`]. Every
//! mutation goes through a method here that delegates calculation to
//! `commands.rs` — views never compute. All methods are headless-testable
//! (no Iced dependency beyond stored data).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use ocalc_engine::{AngleMode, CalculatorEngine, GraphPoint, Value};
use ocalc_history::{HistoryEntry, HistoryStore};

use crate::commands;
use crate::modules;
use crate::theme::{OdsTheme, ThemeMode};

// ---------------------------------------------------------------------------
// Modes
// ---------------------------------------------------------------------------

/// Switchable calculator modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CalcMode {
    /// Basic display + keypad + history.
    #[default]
    Standard,
    /// Standard + trig/log/power keypad.
    Scientific,
    /// Bitwise keypad + base readouts.
    Programmer,
    /// Unit conversion form.
    Converter,
    /// Expression grapher.
    Graph,
    /// Computer algebra (symbolic simplify / calculus / solving).
    Cas,
    /// Physics formula toolkit.
    Physics,
    /// Chemistry formula + periodic table toolkit.
    Chemistry,
    /// Finance calculators.
    Finance,
}

impl CalcMode {
    /// All modes in tab order.
    pub const ALL: [Self; 9] = [
        Self::Standard,
        Self::Scientific,
        Self::Programmer,
        Self::Converter,
        Self::Graph,
        Self::Cas,
        Self::Physics,
        Self::Chemistry,
        Self::Finance,
    ];

    /// Main calculator modes (first toolbar row).
    pub const MAIN: [Self; 5] = [
        Self::Standard,
        Self::Scientific,
        Self::Programmer,
        Self::Converter,
        Self::Graph,
    ];

    /// Scientific category modes (second toolbar row).
    pub const SCIENTIFIC: [Self; 4] = [Self::Cas, Self::Physics, Self::Chemistry, Self::Finance];

    /// Tab label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Scientific => "Scientific",
            Self::Programmer => "Programmer",
            Self::Converter => "Converter",
            Self::Graph => "Graph",
            Self::Cas => "CAS",
            Self::Physics => "Physics",
            Self::Chemistry => "Chemistry",
            Self::Finance => "Finance",
        }
    }

    /// Keyboard shortcut digit (`Ctrl+<digit>`).
    #[must_use]
    pub fn shortcut_digit(self) -> char {
        match self {
            Self::Standard => '1',
            Self::Scientific => '2',
            Self::Programmer => '3',
            Self::Converter => '4',
            Self::Graph => '5',
            Self::Cas => '6',
            Self::Physics => '7',
            Self::Chemistry => '8',
            Self::Finance => '9',
        }
    }
}

impl std::fmt::Display for CalcMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Angle units (UI-level; the engine only knows radians/degrees)
// ---------------------------------------------------------------------------

/// Trig angle unit selected in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AngleUnit {
    /// Engine radians, direct.
    #[default]
    Radians,
    /// Engine degrees, direct.
    Degrees,
    /// Gradians: engine stays in radians; inputs are rewritten with
    /// [`commands::grad_to_radian_expr`] (exact `pi/200` factors, all math in
    /// the engine).
    Gradians,
}

impl AngleUnit {
    /// All units in selector order.
    pub const ALL: [Self; 3] = [Self::Radians, Self::Degrees, Self::Gradians];

    /// Selector label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Radians => "RAD",
            Self::Degrees => "DEG",
            Self::Gradians => "GRAD",
        }
    }

    /// Engine mode backing this unit.
    #[must_use]
    pub fn to_engine(self) -> AngleMode {
        match self {
            Self::Radians | Self::Gradians => AngleMode::Radians,
            Self::Degrees => AngleMode::Degrees,
        }
    }

    /// True when inputs need the gradian rewrite.
    #[must_use]
    pub fn uses_grad(self) -> bool {
        self == Self::Gradians
    }
}

// ---------------------------------------------------------------------------
// Graph state (multiple functions, zoom/pan/grid)
// ---------------------------------------------------------------------------

/// One plotted function.
#[derive(Debug, Clone)]
pub struct GraphFunc {
    /// Stable id for messages.
    pub id: usize,
    /// Expression in `x`.
    pub expr: String,
    /// Palette index (wraps [`crate::ods_theme::colors`] accent presets).
    pub color_idx: usize,
    /// Hidden functions are kept but not sampled/drawn.
    pub visible: bool,
    /// Last samples (empty on error).
    pub samples: Vec<GraphPoint>,
    /// Last sampling error.
    pub error: Option<String>,
}

/// Graph tab state.
#[derive(Debug, Clone)]
pub struct GraphState {
    /// Plotted functions (1–4).
    pub funcs: Vec<GraphFunc>,
    /// Next function id.
    pub next_id: usize,
    /// Range start text.
    pub from_text: String,
    /// Range end text.
    pub to_text: String,
    /// Point count per function.
    pub points: usize,
    /// Zoom multiplier around the range center (1 = fit).
    pub zoom: f32,
    /// Horizontal pan in data units.
    pub pan: f64,
    /// Draw grid + axes.
    pub show_grid: bool,
    /// Maximum functions (keeps sampling bounded).
    pub max_funcs: usize,
    /// Hovered data coordinates `(x, y)` from the canvas, if any.
    pub hover: Option<(f64, f64)>,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            funcs: vec![
                GraphFunc {
                    id: 0,
                    expr: "sin(x)".to_owned(),
                    color_idx: 0,
                    visible: true,
                    samples: Vec::new(),
                    error: None,
                },
                GraphFunc {
                    id: 1,
                    expr: "x^2".to_owned(),
                    color_idx: 2,
                    visible: false,
                    samples: Vec::new(),
                    error: None,
                },
            ],
            next_id: 2,
            from_text: "-10".to_owned(),
            to_text: "10".to_owned(),
            points: 201,
            zoom: 1.0,
            pan: 0.0,
            show_grid: true,
            max_funcs: 4,
            hover: None,
        }
    }
}

impl GraphState {
    /// Effective range after zoom/pan: `(from, to)`.
    #[must_use]
    pub fn effective_range(&self) -> (f64, f64) {
        let from = commands::parse_bound(&self.from_text, -10.0);
        let to = commands::parse_bound(&self.to_text, 10.0);
        let (mut from, mut to) = if from <= to { (from, to) } else { (to, from) };
        let mid = (from + to) / 2.0 + self.pan;
        let span = ((to - from) / f64::from(self.zoom.max(0.1))).max(1e-9);
        from = mid - span / 2.0;
        to = mid + span / 2.0;
        (from, to)
    }

    /// Zoom in (range × 0.8).
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.25).min(64.0);
    }

    /// Zoom out (range × 1.25).
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.25).max(1.0 / 64.0);
    }

    /// Pan by a fraction of the current span.
    pub fn pan_by(&mut self, frac: f64) {
        let (from, to) = self.effective_range();
        self.pan += (to - from) * frac;
    }

    /// Reset zoom/pan.
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Converter state (engine categories + composite speed)
// ---------------------------------------------------------------------------

/// Converter category (engine categories + composite speed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UnitCategoryId {
    /// Engine Length.
    #[default]
    Length,
    /// Engine Mass.
    Mass,
    /// Engine Temperature.
    Temperature,
    /// Engine Time.
    Time,
    /// Engine Data.
    Data,
    /// Engine Energy.
    Energy,
    /// Composite length/time (converted via engine-composed eval).
    Speed,
}

impl UnitCategoryId {
    /// All categories in UI order.
    pub const ALL: [Self; 7] = [
        Self::Length,
        Self::Mass,
        Self::Temperature,
        Self::Time,
        Self::Data,
        Self::Energy,
        Self::Speed,
    ];

    /// Human label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Mass => "Mass",
            Self::Temperature => "Temperature",
            Self::Time => "Time",
            Self::Data => "Data size",
            Self::Energy => "Energy",
            Self::Speed => "Speed",
        }
    }

    /// Symbols for the from/to pickers (engine tables; speed = length set,
    /// the time part comes from [`commands::speed_time_symbols`]).
    #[must_use]
    pub fn symbols(self) -> Vec<String> {
        match self {
            Self::Length => commands::symbols_in_category_public("length"),
            Self::Mass => commands::symbols_in_category_public("mass"),
            Self::Temperature => commands::symbols_in_category_public("temperature"),
            Self::Time => commands::symbols_in_category_public("time"),
            Self::Data => commands::symbols_in_category_public("data"),
            Self::Energy => commands::symbols_in_category_public("energy"),
            Self::Speed => commands::speed_length_symbols(),
        }
    }
}

/// Converter tab state.
#[derive(Debug, Clone)]
pub struct ConverterState {
    /// Selected category.
    pub category: UnitCategoryId,
    /// Numeric input text.
    pub value_text: String,
    /// Source unit symbol.
    pub from: String,
    /// Target unit symbol.
    pub to: String,
    /// Speed source time unit.
    pub speed_from_time: String,
    /// Speed target time unit.
    pub speed_to_time: String,
    /// Last result.
    pub result: Option<f64>,
    /// Last error.
    pub error: Option<String>,
}

impl Default for ConverterState {
    fn default() -> Self {
        Self {
            category: UnitCategoryId::Length,
            value_text: "1".to_owned(),
            from: "km".to_owned(),
            to: "m".to_owned(),
            speed_from_time: "h".to_owned(),
            speed_to_time: "s".to_owned(),
            result: None,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Programmer state
// ---------------------------------------------------------------------------

/// Display base for programmer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumBase {
    /// Binary.
    Bin,
    /// Octal.
    Oct,
    /// Decimal (default).
    #[default]
    Dec,
    /// Hexadecimal.
    Hex,
}

impl NumBase {
    /// All bases in UI order.
    pub const ALL: [Self; 4] = [Self::Bin, Self::Oct, Self::Dec, Self::Hex];

    /// Short label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bin => "BIN",
            Self::Oct => "OCT",
            Self::Dec => "DEC",
            Self::Hex => "HEX",
        }
    }
}

/// Programmer tab state (display-only; all math stays in the engine).
#[derive(Debug, Clone)]
pub struct ProgrammerState {
    /// Emphasized base.
    pub base: NumBase,
    /// Show the interactive bit view.
    pub show_bits: bool,
    /// Interpret integers as signed (two's complement, 64-bit) vs unsigned.
    pub signed: bool,
}

impl Default for ProgrammerState {
    fn default() -> Self {
        Self {
            base: NumBase::Dec,
            show_bits: true,
            signed: true,
        }
    }
}

// ---------------------------------------------------------------------------
// CAS state (symbolic UI; rewrites run in `ocalc-cas` via `commands`)
// ---------------------------------------------------------------------------

/// One CAS history entry.
#[derive(Debug, Clone)]
pub struct CasEntry {
    /// Stable id for messages.
    pub id: usize,
    /// Operation label (`simplify`, `d/dx`, …).
    pub op: String,
    /// Input expression text.
    pub input: String,
    /// Result text.
    pub output: String,
}

/// CAS tab state.
#[derive(Debug, Clone)]
pub struct CasState {
    /// Symbolic expression input.
    pub input: String,
    /// Differentiation / integration variable.
    pub var: String,
    /// Linear-solver `a` (`a·x + b = 0`).
    pub lin_a: String,
    /// Linear-solver `b`.
    pub lin_b: String,
    /// Quadratic-solver `a` (`a·x² + b·x + c = 0`).
    pub quad_a: String,
    /// Quadratic-solver `b`.
    pub quad_b: String,
    /// Quadratic-solver `c`.
    pub quad_c: String,
    /// Last result.
    pub result: Option<String>,
    /// Last error.
    pub error: Option<String>,
    /// Operation history (newest last, capped).
    pub history: Vec<CasEntry>,
    /// Next history id.
    pub next_id: usize,
    /// Maximum history entries kept.
    pub max_history: usize,
    /// Solver panel expanded.
    pub show_solvers: bool,
    /// History panel expanded.
    pub show_history: bool,
}

impl Default for CasState {
    fn default() -> Self {
        Self {
            input: "x^2 + 2*x + 1".to_owned(),
            var: "x".to_owned(),
            lin_a: "2".to_owned(),
            lin_b: "-4".to_owned(),
            quad_a: "1".to_owned(),
            quad_b: "0".to_owned(),
            quad_c: "-4".to_owned(),
            result: None,
            error: None,
            history: Vec::new(),
            next_id: 1,
            max_history: 50,
            show_solvers: true,
            show_history: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Physics state
// ---------------------------------------------------------------------------

/// Physics formula selector (UI-level; computation runs in `ocalc-physics`
/// via `commands::physics_compute`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsFormula {
    /// Force `F = m·a`.
    #[default]
    Force,
    /// Weight `W = m·g`.
    Weight,
    /// Velocity `v = d / t`.
    Velocity,
    /// Acceleration `a = Δv / t`.
    Acceleration,
    /// Momentum `p = m·v`.
    Momentum,
    /// Kinetic energy `E = ½·m·v²`.
    KineticEnergy,
    /// Potential energy `E = m·g·h`.
    PotentialEnergy,
    /// Voltage `V = I·R`.
    Voltage,
    /// Current `I = V / R`.
    Current,
    /// Resistance `R = V / I`.
    Resistance,
    /// Power `P = V·I`.
    PowerVI,
    /// Power `P = I²·R`.
    PowerRI,
    /// Power `P = V²/R`.
    PowerVR,
}

impl PhysicsFormula {
    /// All formulas in selector order.
    pub const ALL: [Self; 13] = [
        Self::Force,
        Self::Weight,
        Self::Velocity,
        Self::Acceleration,
        Self::Momentum,
        Self::KineticEnergy,
        Self::PotentialEnergy,
        Self::Voltage,
        Self::Current,
        Self::Resistance,
        Self::PowerVI,
        Self::PowerRI,
        Self::PowerVR,
    ];

    /// Engine function id used by [`commands::physics_compute`](crate::commands::physics_compute).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Force => "force",
            Self::Weight => "weight",
            Self::Velocity => "velocity",
            Self::Acceleration => "acceleration",
            Self::Momentum => "momentum",
            Self::KineticEnergy => "kinetic_energy",
            Self::PotentialEnergy => "potential_energy",
            Self::Voltage => "voltage",
            Self::Current => "current",
            Self::Resistance => "resistance",
            Self::PowerVI => "power_vi",
            Self::PowerRI => "power_ri",
            Self::PowerVR => "power_vr",
        }
    }

    /// Selector label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Force => "Force (F = m·a)",
            Self::Weight => "Weight (W = m·g)",
            Self::Velocity => "Velocity (v = d/t)",
            Self::Acceleration => "Acceleration (a = Δv/t)",
            Self::Momentum => "Momentum (p = m·v)",
            Self::KineticEnergy => "Kinetic energy (½·m·v²)",
            Self::PotentialEnergy => "Potential energy (m·g·h)",
            Self::Voltage => "Voltage (V = I·R)",
            Self::Current => "Current (I = V/R)",
            Self::Resistance => "Resistance (R = V/I)",
            Self::PowerVI => "Power (P = V·I)",
            Self::PowerRI => "Power (P = I²·R)",
            Self::PowerVR => "Power (P = V²/R)",
        }
    }

    /// First input field label.
    #[must_use]
    pub fn a_label(self) -> &'static str {
        match self {
            Self::Force => "mass (kg)",
            Self::Weight => "mass (kg)",
            Self::Velocity => "distance (m)",
            Self::Acceleration => "Δv (m/s)",
            Self::Momentum => "mass (kg)",
            Self::KineticEnergy => "mass (kg)",
            Self::PotentialEnergy => "mass (kg)",
            Self::Voltage => "current (A)",
            Self::Current => "voltage (V)",
            Self::Resistance => "voltage (V)",
            Self::PowerVI => "voltage (V)",
            Self::PowerRI => "current (A)",
            Self::PowerVR => "voltage (V)",
        }
    }

    /// Second input field label (`None` for single-input formulas).
    #[must_use]
    pub fn b_label(self) -> Option<&'static str> {
        match self {
            Self::Weight => None,
            Self::Force => Some("acceleration (m/s²)"),
            Self::Velocity => Some("time (s)"),
            Self::Acceleration => Some("time (s)"),
            Self::Momentum => Some("velocity (m/s)"),
            Self::KineticEnergy => Some("velocity (m/s)"),
            Self::PotentialEnergy => Some("height (m)"),
            Self::Voltage => Some("resistance (Ω)"),
            Self::Current => Some("resistance (Ω)"),
            Self::Resistance => Some("current (A)"),
            Self::PowerVI => Some("current (A)"),
            Self::PowerRI => Some("resistance (Ω)"),
            Self::PowerVR => Some("resistance (Ω)"),
        }
    }
}

impl std::fmt::Display for PhysicsFormula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Physics tab state.
#[derive(Debug, Clone)]
pub struct PhysicsState {
    /// Selected formula.
    pub formula: PhysicsFormula,
    /// First input text.
    pub a_text: String,
    /// Second input text.
    pub b_text: String,
    /// Last formatted value.
    pub result: Option<String>,
    /// Result unit.
    pub unit: String,
    /// Last error.
    pub error: Option<String>,
    /// Constants reference panel expanded.
    pub show_constants: bool,
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self {
            formula: PhysicsFormula::Force,
            a_text: "2".to_owned(),
            b_text: "3".to_owned(),
            result: None,
            unit: String::new(),
            error: None,
            show_constants: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Chemistry state
// ---------------------------------------------------------------------------

/// Chemistry tab state.
#[derive(Debug, Clone)]
pub struct ChemistryState {
    /// Chemical formula input.
    pub formula: String,
    /// Last molar mass (g/mol).
    pub result_mass: Option<f64>,
    /// Last per-element breakdown.
    pub breakdown: Vec<crate::commands::ChemistryComponent>,
    /// Last error.
    pub error: Option<String>,
    /// Selected element symbol for the info panel.
    pub selected: String,
    /// Periodic-table search query.
    pub table_search: String,
}

impl Default for ChemistryState {
    fn default() -> Self {
        Self {
            formula: "H2O".to_owned(),
            result_mass: None,
            breakdown: Vec::new(),
            error: None,
            selected: "O".to_owned(),
            table_search: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Finance state (four stacked calculators; math runs in `ocalc-finance`)
// ---------------------------------------------------------------------------

/// Finance tab state: one field group per calculator plus per-tool outputs.
#[derive(Debug, Clone)]
pub struct FinanceState {
    /// Compound: principal.
    pub compound_p: String,
    /// Compound: annual rate (%).
    pub compound_r: String,
    /// Compound: periods.
    pub compound_n: String,
    /// Compound: nominal rate (%) for the effective-rate readout.
    pub compound_nominal: String,
    /// Compound: compounds per year.
    pub compound_m: String,
    /// Compound future value.
    pub compound_result: Option<f64>,
    /// Effective annual rate (fraction).
    pub compound_effective: Option<f64>,
    /// Compound error.
    pub compound_error: Option<String>,
    /// Effective-rate error.
    pub compound_effective_error: Option<String>,
    /// Loan: principal.
    pub loan_p: String,
    /// Loan: APR (%).
    pub loan_apr: String,
    /// Loan: months.
    pub loan_n: String,
    /// Loan monthly payment.
    pub loan_payment: Option<f64>,
    /// Loan lifetime total.
    pub loan_total: Option<f64>,
    /// Loan lifetime interest.
    pub loan_interest: Option<f64>,
    /// Loan error.
    pub loan_error: Option<String>,
    /// Growth value: start.
    pub growth_start: String,
    /// Growth value: rate (%).
    pub growth_rate: String,
    /// Growth value: periods.
    pub growth_n: String,
    /// Growth end value.
    pub growth_result: Option<f64>,
    /// Growth value error.
    pub growth_error: Option<String>,
    /// Growth rate: start.
    pub rate_start: String,
    /// Growth rate: end.
    pub rate_end: String,
    /// Growth rate: periods.
    pub rate_n: String,
    /// Growth rate (fraction).
    pub rate_result: Option<f64>,
    /// Growth rate error.
    pub rate_error: Option<String>,
    /// Percent: base.
    pub percent_base: String,
    /// Percent: percent.
    pub percent_pct: String,
    /// Percent result.
    pub percent_result: Option<f64>,
    /// Percent error.
    pub percent_error: Option<String>,
    /// Percent change: old value.
    pub change_old: String,
    /// Percent change: new value.
    pub change_new: String,
    /// Percent change (percentage points).
    pub change_result: Option<f64>,
    /// Percent change error.
    pub change_error: Option<String>,
}

impl Default for FinanceState {
    fn default() -> Self {
        Self {
            compound_p: "1000".to_owned(),
            compound_r: "5".to_owned(),
            compound_n: "10".to_owned(),
            compound_nominal: "12".to_owned(),
            compound_m: "12".to_owned(),
            compound_result: None,
            compound_effective: None,
            compound_error: None,
            compound_effective_error: None,
            loan_p: "10000".to_owned(),
            loan_apr: "6".to_owned(),
            loan_n: "12".to_owned(),
            loan_payment: None,
            loan_total: None,
            loan_interest: None,
            loan_error: None,
            growth_start: "100".to_owned(),
            growth_rate: "10".to_owned(),
            growth_n: "2".to_owned(),
            growth_result: None,
            growth_error: None,
            rate_start: "100".to_owned(),
            rate_end: "121".to_owned(),
            rate_n: "2".to_owned(),
            rate_result: None,
            rate_error: None,
            percent_base: "200".to_owned(),
            percent_pct: "15".to_owned(),
            percent_result: None,
            percent_error: None,
            change_old: "100".to_owned(),
            change_new: "120".to_owned(),
            change_result: None,
            change_error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Toasts, status, settings
// ---------------------------------------------------------------------------

/// Transient notification (error or info), dismissed by the user.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Message text.
    pub message: String,
    /// True for errors (danger color), false for info.
    pub is_error: bool,
}

/// Evaluation status for the status bar.
#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    /// Last evaluation time in microseconds.
    pub last_duration_us: Option<u64>,
    /// Total evaluations this session.
    pub eval_count: u64,
}

/// Persisted user settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Versioned config format (see [`SETTINGS_FORMAT_VERSION`]).
    /// Missing in pre-1.0 files → defaults to v1 and migrates forward.
    #[serde(default = "default_settings_format_version")]
    pub config_version: u32,
    /// Display precision (decimal places).
    pub precision: u8,
    /// Trig angle unit.
    pub angle: AngleUnit,
    /// Theme mode.
    pub theme: ThemeMode,
    /// Accent color index into accent presets (None = mode default).
    pub accent_idx: Option<usize>,
    /// Font scale multiplier.
    pub font_scale: f32,
    /// Maximum history entries kept.
    pub history_capacity: usize,
    /// Mode selected at startup.
    pub startup_mode: CalcMode,
    /// History sidebar visible.
    pub show_history: bool,
    /// Settings panel visible.
    pub show_settings: bool,
    /// Result number format.
    #[serde(default)]
    pub number_format: NumberFormat,
    /// Trim history to capacity automatically on submit.
    #[serde(default = "default_true")]
    pub history_auto_trim: bool,
    /// Starred calculations (persisted favorites).
    #[serde(default)]
    pub favorites: Vec<FavoriteEntry>,
}

/// Number display format for results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumberFormat {
    /// Classic display (rounded fixed-point, exact integers untouched).
    #[default]
    Standard,
    /// Scientific notation for inexact results (`1.5e4`).
    Scientific,
}

impl NumberFormat {
    /// Selector label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Scientific => "Scientific",
        }
    }
}

impl std::fmt::Display for NumberFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Serde default for fields that predate them in older settings files.
fn default_true() -> bool {
    true
}

/// Maximum starred calculations kept.
pub const MAX_FAVORITES: usize = 50;

/// One starred calculation (expression + rendered result snapshot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteEntry {
    /// Stable id for messages.
    pub id: u64,
    /// Expression text as typed.
    pub expression: String,
    /// Rendered result at favorite time.
    pub result: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            config_version: SETTINGS_FORMAT_VERSION,
            precision: 6,
            angle: AngleUnit::Radians,
            theme: ThemeMode::Dark,
            accent_idx: None,
            font_scale: 1.0,
            history_capacity: 500,
            startup_mode: CalcMode::Standard,
            show_history: true,
            show_settings: false,
            number_format: NumberFormat::Standard,
            history_auto_trim: true,
            favorites: Vec::new(),
        }
    }
}

/// Current versioned settings format. Bumped whenever a field is added or
/// reinterpreted; [`Settings::load`] migrates older files forward and resets
/// unknown future files to defaults (never crashes on old configs).
pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// Serde default for [`Settings::config_version`].
fn default_settings_format_version() -> u32 {
    SETTINGS_FORMAT_VERSION
}

impl Settings {
    /// Load from `path` (JSON). Returns default on any failure.
    ///
    /// Version handling: files without a version stamp (pre-1.0) are treated
    /// as v1 and clamped; files stamped newer than
    /// [`SETTINGS_FORMAT_VERSION`] reset to defaults (a newer app wrote
    /// them); v1 files load normally.
    #[must_use]
    pub fn load(path: &std::path::Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        let parsed: Self = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        if parsed.config_version > SETTINGS_FORMAT_VERSION {
            return Self::default();
        }
        parsed.clamped()
    }

    /// Save to `path` (creates parent dirs). Best-effort: errors as strings.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Clamp every field to its valid range.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        // Forward migration: stamp the current format version.
        self.config_version = SETTINGS_FORMAT_VERSION;
        self.precision = self.precision.min(commands::MAX_PRECISION);
        self.font_scale = self.font_scale.clamp(0.875, 1.5);
        self.history_capacity = self.history_capacity.clamp(10, 10_000);
        if let Some(i) = self.accent_idx {
            if i >= crate::ods_theme::ACCENT_PRESETS.len() {
                self.accent_idx = None;
            }
        }
        if self.favorites.len() > MAX_FAVORITES {
            let drop = self.favorites.len() - MAX_FAVORITES;
            self.favorites.drain(..drop);
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Full application state
// ---------------------------------------------------------------------------

/// Full application state.
#[derive(Debug)]
pub struct AppState {
    /// Stable engine (only calculation backend).
    pub engine: CalculatorEngine,
    /// Active mode tab.
    pub mode: CalcMode,
    /// Current expression input.
    pub input: String,
    /// Formatted result display.
    pub display: String,
    /// Error message (caret rendering included).
    pub error: Option<String>,
    /// Last successful value (for programmer bases).
    pub last_result: Option<Value>,
    /// Calculator memory register (M+/M-/MR/MC).
    pub memory: Option<Value>,
    /// Calculation log.
    pub history: HistoryStore,
    /// History search query.
    pub history_search: String,
    /// Display precision (decimal places).
    pub precision: u8,
    /// Trig angle unit.
    pub angle: AngleUnit,
    /// ODS theme.
    pub theme: OdsTheme,
    /// User settings (persisted).
    pub settings: Settings,
    /// Transient notification.
    pub toast: Option<Toast>,
    /// Session evaluation stats.
    pub status: StatusInfo,
    /// Graph tab.
    pub graph: GraphState,
    /// Converter tab.
    pub converter: ConverterState,
    /// Programmer tab.
    pub programmer: ProgrammerState,
    /// CAS tab.
    pub cas: CasState,
    /// Physics tab.
    pub physics: PhysicsState,
    /// Chemistry tab.
    pub chemistry: ChemistryState,
    /// Finance tab.
    pub finance: FinanceState,
    /// Function search query (scientific + built-in function finder).
    pub function_search: String,
    /// Disabled scientific module ids (empty = all enabled).
    pub disabled_modules: HashSet<String>,
    /// Starred calculations.
    pub favorites: Vec<FavoriteEntry>,
    /// Next favorite id.
    pub next_fav_id: u64,
    /// Result number format.
    pub number_format: NumberFormat,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Fresh state with default engine, settings and tabs.
    #[must_use]
    pub fn new() -> Self {
        Self::with_settings(Settings::default())
    }

    /// Fresh state with explicit settings (startup mode + caps applied).
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        let settings = settings.clamped();
        let mut theme = OdsTheme::with_mode(settings.theme);
        theme.set_font_scale(settings.font_scale);
        if let Some(i) = settings.accent_idx {
            theme.set_accent(crate::ods_theme::ACCENT_PRESETS[i]);
        }
        let favorites = settings.favorites.clone();
        let next_fav_id = favorites.iter().map(|f| f.id).max().unwrap_or(0) + 1;
        let number_format = settings.number_format;
        let mut s = Self {
            engine: CalculatorEngine::new(),
            mode: settings.startup_mode,
            input: String::new(),
            display: String::new(),
            error: None,
            last_result: None,
            memory: None,
            history: HistoryStore::with_capacity(settings.history_capacity),
            history_search: String::new(),
            precision: settings.precision,
            angle: settings.angle,
            theme,
            settings,
            toast: None,
            status: StatusInfo::default(),
            graph: GraphState::default(),
            converter: ConverterState::default(),
            programmer: ProgrammerState::default(),
            cas: CasState::default(),
            physics: PhysicsState::default(),
            chemistry: ChemistryState::default(),
            finance: FinanceState::default(),
            function_search: String::new(),
            disabled_modules: HashSet::new(),
            favorites,
            next_fav_id,
            number_format,
        };
        s.engine.set_angle_mode(s.angle.to_engine());
        // Phase 6: discover scientific modules through the stable plugin API.
        // The engine core is untouched — registration only.
        modules::register_scientific_modules(&mut s.engine);
        s.refresh_graph();
        s.refresh_converter();
        s.refresh_physics();
        s.refresh_chemistry();
        s.refresh_finance();
        s
    }

    /// Sync a settings change back into live state (theme/engine/caps).
    pub fn apply_settings(&mut self) {
        self.precision = self.settings.precision;
        self.angle = self.settings.angle;
        self.number_format = self.settings.number_format;
        self.engine.set_angle_mode(self.angle.to_engine());
        let mut theme = OdsTheme::with_mode(self.settings.theme);
        theme.set_font_scale(self.settings.font_scale);
        if let Some(i) = self.settings.accent_idx {
            theme.set_accent(crate::ods_theme::ACCENT_PRESETS[i]);
        }
        self.theme = theme;
        if let Some(v) = self.last_result.clone() {
            self.display = self.format_result(&v);
        }
        self.enforce_history_capacity();
        self.refresh_graph();
    }

    /// Set the full angle unit (RAD/DEG/GRAD).
    pub fn set_angle_unit(&mut self, unit: AngleUnit) {
        self.angle = unit;
        self.settings.angle = unit;
        self.engine.set_angle_mode(unit.to_engine());
    }

    /// Set display precision (clamped to `MAX_PRECISION`).
    pub fn set_precision(&mut self, precision: u8) {
        self.precision = precision.min(commands::MAX_PRECISION);
        self.settings.precision = self.precision;
        // Re-render the current result at the new precision.
        if let Some(v) = self.last_result.clone() {
            self.display = self.format_result(&v);
        }
    }

    /// Set the result number format (mirrored into settings for persistence).
    pub fn set_number_format(&mut self, format: NumberFormat) {
        self.number_format = format;
        self.settings.number_format = format;
        // Re-render the current result in the new format.
        if let Some(v) = self.last_result.clone() {
            self.display = self.format_result(&v);
        }
    }

    /// Format a result [`Value`] with the active precision + number format.
    #[must_use]
    pub fn format_result(&self, value: &Value) -> String {
        commands::format_number(
            value,
            self.precision,
            self.number_format == NumberFormat::Scientific,
        )
    }

    /// Set theme (resets accent colors to mode defaults).
    pub fn set_theme(&mut self, mode: ThemeMode) {
        self.theme.set_mode(mode);
        self.settings.theme = mode;
        self.settings.accent_idx = None;
    }

    /// Append a literal to the input (keypad / keyboard).
    pub fn append(&mut self, s: &str) {
        self.input.push_str(s);
    }

    /// Delete the last character.
    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Clear the input entry only (CE): keeps result/error for reference.
    pub fn clear_entry(&mut self) {
        self.input.clear();
    }

    /// Clear input, display and error (keeps history/settings).
    pub fn clear_all(&mut self) {
        self.input.clear();
        self.display.clear();
        self.error = None;
        self.last_result = None;
    }

    /// Negate: empty input → `-`; leading `-` toggles off; else prepend `-`.
    pub fn negate(&mut self) {
        if self.input.starts_with('-') {
            self.input.remove(0);
        } else {
            self.input.insert(0, '-');
        }
    }

    /// Evaluate the current input, update display/error/history/status.
    pub fn submit(&mut self) {
        let expr_text = self.input.trim().to_owned();
        if expr_text.is_empty() {
            return;
        }
        // GRAD: rewrite angles at the UI boundary; the engine still evaluates.
        let effective = if self.angle.uses_grad() {
            commands::grad_to_radian_expr(&expr_text)
        } else {
            expr_text.clone()
        };
        let started = std::time::Instant::now();
        let outcome = commands::evaluate(&mut self.engine, &effective, self.precision);
        self.status.eval_count += 1;
        self.status.last_duration_us =
            Some(started.elapsed().as_micros().min(u64::MAX as u128) as u64);
        match outcome.result {
            Some(v) => {
                // Re-render through the active number format (evaluate uses
                // standard formatting internally).
                self.display = self.format_result(&v);
                self.error = None;
                self.toast = None;
                self.last_result = Some(v.clone());
                // History records what the user typed, not the rewrite.
                self.history.push(&expr_text, &self.display);
                self.enforce_history_capacity();
            }
            None => {
                self.display.clear();
                self.last_result = None;
                self.error = outcome.error.clone();
                self.toast = outcome.error.map(|m| Toast {
                    message: m.lines().next().unwrap_or("evaluation failed").to_owned(),
                    is_error: true,
                });
            }
        }
    }

    /// Drop oldest entries beyond the configured capacity (ids preserved).
    /// Skipped when automatic cleanup is disabled in settings.
    fn enforce_history_capacity(&mut self) {
        if !self.settings.history_auto_trim {
            return;
        }
        let cap = self.settings.history_capacity;
        if self.history.len() > cap {
            let keep: std::collections::HashSet<u64> =
                self.history.last(cap).iter().map(|e| e.id).collect();
            match commands::rebuild_history(&self.history, &keep, Some(cap)) {
                Ok(back) => self.history = back,
                Err(msg) => {
                    self.toast = Some(Toast {
                        message: msg,
                        is_error: true,
                    });
                }
            }
        }
    }

    /// History filtered by the search query (newest first, capped at 100).
    #[must_use]
    pub fn filtered_history(&self) -> Vec<HistoryEntry> {
        if self.history_search.trim().is_empty() {
            self.history.last(100)
        } else {
            self.history
                .search(self.history_search.trim())
                .into_iter()
                .rev()
                .take(100)
                .collect()
        }
    }

    /// Reuse a history entry: load its expression into the input.
    pub fn reuse_history(&mut self, id: u64) {
        if let Some(entry) = self.history.all().iter().find(|e| e.id == id) {
            self.input = entry.expression.clone();
            self.error = None;
        }
    }

    /// Delete one history entry (ids of the rest are preserved).
    pub fn delete_history(&mut self, id: u64) {
        let keep: std::collections::HashSet<u64> = self
            .history
            .all()
            .iter()
            .filter(|e| e.id != id)
            .map(|e| e.id)
            .collect();
        match commands::rebuild_history(&self.history, &keep, None) {
            Ok(back) => {
                self.history = back;
                self.notify(format!("Deleted entry #{id}"), false);
            }
            Err(msg) => {
                self.toast = Some(Toast {
                    message: msg,
                    is_error: true,
                });
            }
        }
    }

    /// Clear history (ids stay monotonic per store contract).
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.notify("History cleared".to_owned(), false);
    }

    /// Show a transient notification.
    pub fn notify(&mut self, message: String, is_error: bool) {
        self.toast = Some(Toast { message, is_error });
    }

    /// Dismiss the notification.
    pub fn dismiss_toast(&mut self) {
        self.toast = None;
    }

    // -- Memory (MC/MR/M+/M-) --------------------------------------------

    /// Store the current result (or evaluate the input first).
    pub fn memory_store(&mut self) {
        if self.last_result.is_none() && !self.input.trim().is_empty() {
            self.submit();
        }
        match self.last_result.clone() {
            Some(v) => {
                self.memory = Some(v);
                self.notify("Stored to memory".to_owned(), false);
            }
            None => self.notify("Nothing to store".to_owned(), true),
        }
    }

    /// Clear memory.
    pub fn memory_clear(&mut self) {
        self.memory = None;
        self.notify("Memory cleared".to_owned(), false);
    }

    /// Recall memory into the input.
    pub fn memory_recall(&mut self) {
        match self.memory.clone() {
            Some(v) => {
                self.input = self.format_result(&v);
                self.error = None;
            }
            None => self.notify("Memory is empty".to_owned(), true),
        }
    }

    /// `M+` / `M-`: combine the evaluated input into memory via public
    /// [`Value`] arithmetic. Empty memory behaves as zero.
    pub fn memory_combine(&mut self, add: bool) {
        if self.last_result.is_none() && !self.input.trim().is_empty() {
            self.submit();
        }
        let current = match self.last_result.clone() {
            Some(v) => v,
            None => {
                self.notify("Nothing to add".to_owned(), true);
                return;
            }
        };
        let base = self.memory.clone().unwrap_or(Value::Integer(0));
        let res = if add {
            commands::memory_add(&base, &current)
        } else {
            commands::memory_sub(&base, &current)
        };
        match res {
            Ok(v) => {
                self.memory = Some(v);
                self.notify("Memory updated".to_owned(), false);
            }
            Err(msg) => self.notify(msg, true),
        }
    }

    // -- Graph ------------------------------------------------------------

    /// Re-sample all visible graph functions from current tab state.
    pub fn refresh_graph(&mut self) {
        // Hover coordinates refer to old samples — drop them.
        self.graph_clear_hover();
        let (from, to) = self.graph.effective_range();
        for func in &mut self.graph.funcs {
            if !func.visible {
                func.samples.clear();
                func.error = None;
                continue;
            }
            let expr = if self.angle.uses_grad() {
                commands::grad_to_radian_expr(func.expr.trim())
            } else {
                func.expr.trim().to_owned()
            };
            match commands::sample_graph_points(&self.engine, &expr, from, to, self.graph.points) {
                Ok(pts) => {
                    func.samples = pts;
                    func.error = None;
                }
                Err(msg) => {
                    func.samples.clear();
                    func.error = Some(msg);
                }
            }
        }
    }

    /// Add a function slot (up to `max_funcs`).
    pub fn graph_add_func(&mut self) {
        if self.graph.funcs.len() >= self.graph.max_funcs {
            self.notify(format!("At most {} functions", self.graph.max_funcs), true);
            return;
        }
        let id = self.graph.next_id;
        self.graph.next_id += 1;
        self.graph.funcs.push(GraphFunc {
            id,
            expr: "x".to_owned(),
            color_idx: id % crate::ods_theme::ACCENT_PRESETS.len().max(1),
            visible: true,
            samples: Vec::new(),
            error: None,
        });
        self.refresh_graph();
    }

    /// Remove a function slot (keeps at least one).
    pub fn graph_remove_func(&mut self, id: usize) {
        if self.graph.funcs.len() <= 1 {
            self.notify("At least one function is required".to_owned(), true);
            return;
        }
        self.graph.funcs.retain(|f| f.id != id);
        self.refresh_graph();
    }

    /// Toggle a function's visibility.
    pub fn graph_toggle_func(&mut self, id: usize) {
        if let Some(f) = self.graph.funcs.iter_mut().find(|f| f.id == id) {
            f.visible = !f.visible;
        }
        self.refresh_graph();
    }

    // -- Converter --------------------------------------------------------

    /// Re-run the unit conversion from current tab state.
    pub fn refresh_converter(&mut self) {
        if self.converter.category == UnitCategoryId::Speed {
            let c = self.converter.clone();
            match commands::convert_speed(
                &mut self.engine,
                &c.value_text,
                &c.from,
                &c.speed_from_time,
                &c.to,
                &c.speed_to_time,
            ) {
                Ok(v) => {
                    self.converter.result = Some(v);
                    self.converter.error = None;
                }
                Err(msg) => {
                    self.converter.result = None;
                    self.converter.error = Some(msg);
                }
            }
            return;
        }
        match commands::convert_units(
            &self.converter.value_text,
            self.converter.from.trim(),
            self.converter.to.trim(),
        ) {
            Ok(v) => {
                self.converter.result = Some(v);
                self.converter.error = None;
            }
            Err(msg) => {
                self.converter.result = None;
                self.converter.error = Some(msg);
            }
        }
    }

    /// Switch converter category, resetting units to sane defaults.
    pub fn set_converter_category(&mut self, category: UnitCategoryId) {
        self.converter.category = category;
        let syms = category.symbols();
        let pick = |want: &[&str]| {
            want.iter()
                .find_map(|w| syms.iter().find(|s| s.eq_ignore_ascii_case(w)).cloned())
                .or_else(|| syms.first().cloned())
                .unwrap_or_default()
        };
        match category {
            UnitCategoryId::Length => {
                self.converter.from = pick(&["km"]);
                self.converter.to = pick(&["m"]);
            }
            UnitCategoryId::Mass => {
                self.converter.from = pick(&["kg"]);
                self.converter.to = pick(&["g"]);
            }
            UnitCategoryId::Temperature => {
                self.converter.from = pick(&["C", "celsius"]);
                self.converter.to = pick(&["F", "fahrenheit"]);
            }
            UnitCategoryId::Time => {
                self.converter.from = pick(&["h", "hour"]);
                self.converter.to = pick(&["min", "minute"]);
            }
            UnitCategoryId::Data => {
                self.converter.from = pick(&["MB", "megabyte"]);
                self.converter.to = pick(&["MiB", "mebibyte"]);
            }
            UnitCategoryId::Energy => {
                self.converter.from = pick(&["kWh"]);
                self.converter.to = pick(&["J", "joule"]);
            }
            UnitCategoryId::Speed => {
                self.converter.from = pick(&["km"]);
                self.converter.to = pick(&["m"]);
                self.converter.speed_from_time = "h".to_owned();
                self.converter.speed_to_time = "s".to_owned();
            }
        }
        self.refresh_converter();
    }

    // -- Programmer ---------------------------------------------------------

    /// Toggle bit `i` of the current integer result by evaluating a rebuilt
    /// binary literal through the engine (no UI-side bit math).
    pub fn toggle_bit(&mut self, i: u32) {
        let current = match self.last_result.clone() {
            Some(v) => v,
            None => {
                self.notify("Evaluate an integer first".to_owned(), true);
                return;
            }
        };
        let bits = match current.as_bigint() {
            Ok(b) => b,
            Err(_) => {
                self.notify("Bit view needs an integer result".to_owned(), true);
                return;
            }
        };
        use num_traits::{One, Zero};
        let bit = num_bigint::BigInt::one() << (i as usize);
        let is_set = (&bits & &bit) != num_bigint::BigInt::zero();
        let next = if is_set { bits - &bit } else { bits + &bit };
        // Negative intermediates stay exact through the engine.
        self.input = next.to_string();
        self.submit();
    }

    // -- Scientific modules (Phase 6: discovery / enable / search) -----------

    /// True when scientific module `id` is currently enabled.
    #[must_use]
    pub fn is_module_enabled(&self, id: &str) -> bool {
        !self.disabled_modules.contains(id)
    }

    /// Enable or disable a scientific module by id.
    ///
    /// Disabling unregisters it from the engine registry; enabling rebuilds
    /// it through the [`modules::create_module`] factory and re-registers.
    /// Unknown ids notify an error toast and do nothing.
    pub fn set_module_enabled(&mut self, id: &str, enabled: bool) {
        if !modules::SCIENTIFIC_MODULE_IDS.contains(&id) {
            self.notify(format!("Unknown module {id:?}"), true);
            return;
        }
        if enabled {
            if self.disabled_modules.remove(id) {
                match modules::create_module(id) {
                    Some(m) => {
                        // Checked registration: version mismatches produce a
                        // clear message instead of a generic failure.
                        // Already-registered (e.g. double toggle) counts as
                        // success. Failures surface via toast (no silent
                        // failures); file logging happens at startup in
                        // `modules::register_scientific_modules`.
                        match self
                            .engine
                            .register_module_checked(m, ocalc_engine::ENGINE_PLUGIN_API_VERSION)
                        {
                            Ok(()) => {
                                self.notify(format!("Module {id} enabled"), false);
                            }
                            Err(ocalc_modules::ModuleError::AlreadyRegistered(_)) => {
                                self.notify(format!("Module {id} enabled"), false);
                            }
                            Err(e) => {
                                self.notify(format!("Cannot enable {id}: {e}"), true);
                            }
                        }
                    }
                    None => self.notify(format!("Unknown module {id:?}"), true),
                }
            }
        } else if self.disabled_modules.insert(id.to_owned()) {
            // `unregister_module` returns false when already absent — still disabled.
            let _ = self.engine.unregister_module(id);
            self.notify(format!("Module {id} disabled"), false);
        }
    }

    /// Toggle a scientific module's enabled state.
    pub fn toggle_module(&mut self, id: &str) {
        let enabled = self.is_module_enabled(id);
        self.set_module_enabled(id, !enabled);
    }

    /// All known scientific modules as `(id, description, functions, enabled)`.
    #[must_use]
    pub fn scientific_modules(&self) -> Vec<(String, String, Vec<String>, bool)> {
        let mut out = Vec::new();
        for id in modules::SCIENTIFIC_MODULE_IDS {
            let funcs = modules::create_module(id)
                .map(|m| m.functions())
                .unwrap_or_default();
            out.push((
                id.to_owned(),
                modules::module_description(id).to_owned(),
                funcs,
                self.is_module_enabled(id),
            ));
        }
        out
    }

    /// Functions matching the current [`Self::function_search`] query
    /// (case-insensitive substring; empty query returns all, capped at 100).
    #[must_use]
    pub fn filtered_functions(&self) -> Vec<String> {
        modules::search_functions(&self.engine, &self.function_search)
            .into_iter()
            .take(100)
            .collect()
    }

    // -- CAS (Phase 7: symbolic UI over `ocalc-cas`) --------------------------

    /// Record a successful CAS operation in the expression history.
    fn cas_push_history(&mut self, op: &str, input: &str, output: &str) {
        let id = self.cas.next_id;
        self.cas.next_id += 1;
        self.cas.history.push(CasEntry {
            id,
            op: op.to_owned(),
            input: input.to_owned(),
            output: output.to_owned(),
        });
        while self.cas.history.len() > self.cas.max_history {
            self.cas.history.remove(0);
        }
    }

    /// Apply a symbolic rewrite to the expression input.
    fn cas_apply(&mut self, op: &str, outcome: Result<String, String>) {
        match outcome {
            Ok(output) => {
                let input = self.cas.input.trim().to_owned();
                self.cas.result = Some(output.clone());
                self.cas.error = None;
                self.cas_push_history(op, &input, &output);
            }
            Err(msg) => {
                self.cas.result = None;
                self.cas.error = Some(msg);
            }
        }
    }

    /// Simplify the expression input (`x + x → 2*x`).
    pub fn cas_simplify(&mut self) {
        let src = self.cas.input.clone();
        self.cas_apply("simplify", commands::cas_simplify(&src));
    }

    /// Expand the expression input (`(x+1)^2 → x^2 + 2*x + 1`).
    pub fn cas_expand(&mut self) {
        let src = self.cas.input.clone();
        self.cas_apply("expand", commands::cas_expand(&src));
    }

    /// Factor the expression input.
    pub fn cas_factor(&mut self) {
        let src = self.cas.input.clone();
        self.cas_apply("factor", commands::cas_factor(&src));
    }

    /// Differentiate the expression input with respect to `var`.
    pub fn cas_differentiate(&mut self) {
        let (src, var) = (self.cas.input.clone(), self.cas.var.clone());
        self.cas_apply("d/dx", commands::cas_differentiate(&src, &var));
    }

    /// Integrate the expression input with respect to `var`.
    pub fn cas_integrate(&mut self) {
        let (src, var) = (self.cas.input.clone(), self.cas.var.clone());
        self.cas_apply("∫dx", commands::cas_integrate(&src, &var));
    }

    /// Solve `a·x + b = 0` from the linear fields.
    pub fn cas_solve_linear(&mut self) {
        let (a, b) = (self.cas.lin_a.clone(), self.cas.lin_b.clone());
        match commands::cas_solve_linear(&a, &b) {
            Ok(output) => {
                self.cas.result = Some(output.clone());
                self.cas.error = None;
                self.cas_push_history("solve", &format!("{a}·x + {b} = 0"), &output);
            }
            Err(msg) => {
                self.cas.result = None;
                self.cas.error = Some(msg);
            }
        }
    }

    /// Solve `a·x² + b·x + c = 0` from the quadratic fields.
    pub fn cas_solve_quadratic(&mut self) {
        let (a, b, c) = (
            self.cas.quad_a.clone(),
            self.cas.quad_b.clone(),
            self.cas.quad_c.clone(),
        );
        match commands::cas_solve_quadratic(&a, &b, &c) {
            Ok(output) => {
                self.cas.result = Some(output.clone());
                self.cas.error = None;
                self.cas_push_history("solve", &format!("{a}·x² + {b}·x + {c} = 0"), &output);
            }
            Err(msg) => {
                self.cas.result = None;
                self.cas.error = Some(msg);
            }
        }
    }

    /// Clear the CAS expression history.
    pub fn cas_clear_history(&mut self) {
        self.cas.history.clear();
        self.notify("CAS history cleared".to_owned(), false);
    }

    /// Reuse a CAS history entry: load its input back into the editor.
    pub fn cas_reuse(&mut self, id: usize) {
        if let Some(entry) = self.cas.history.iter().find(|e| e.id == id) {
            self.cas.input = entry.input.clone();
            self.cas.error = None;
        }
    }

    // -- Physics (Phase 7: formula UI over `ocalc-physics`) -------------------

    /// Recompute the physics result from the current formula + inputs.
    pub fn refresh_physics(&mut self) {
        let p = &self.physics;
        match commands::physics_compute(p.formula.id(), &p.a_text, &p.b_text, self.precision) {
            Ok((value, unit)) => {
                self.physics.result = Some(value);
                self.physics.unit = unit.to_owned();
                self.physics.error = None;
            }
            Err(msg) => {
                self.physics.result = None;
                self.physics.unit.clear();
                self.physics.error = Some(msg);
            }
        }
    }

    /// Select a physics formula (resets inputs to a worked example).
    pub fn set_physics_formula(&mut self, formula: PhysicsFormula) {
        self.physics.formula = formula;
        // Sensible defaults per arity so the panel never starts blank.
        let (a, b) = match formula {
            PhysicsFormula::Weight => ("10".to_owned(), String::new()),
            _ => ("2".to_owned(), "3".to_owned()),
        };
        self.physics.a_text = a;
        self.physics.b_text = b;
        self.refresh_physics();
    }

    // -- Chemistry (Phase 7: formula UI over `ocalc-chemistry`) ---------------

    /// Re-analyze the formula input (molar mass + breakdown).
    pub fn refresh_chemistry(&mut self) {
        let formula = self.chemistry.formula.clone();
        match commands::chemistry_analyze(&formula) {
            Ok(rep) => {
                self.chemistry.result_mass = Some(rep.mass);
                self.chemistry.breakdown = rep.components;
                self.chemistry.error = None;
            }
            Err(msg) => {
                self.chemistry.result_mass = None;
                self.chemistry.breakdown.clear();
                self.chemistry.error = Some(msg);
            }
        }
    }

    /// Periodic-table entries matching the current search query.
    #[must_use]
    pub fn chemistry_filtered_table(&self) -> Vec<commands::ElementCard> {
        commands::filter_elements(&self.chemistry.table_search)
    }

    /// Card for the currently selected element, if it exists.
    #[must_use]
    pub fn chemistry_selected_card(&self) -> Option<commands::ElementCard> {
        commands::chemistry_element(&self.chemistry.selected)
    }

    // -- Finance (Phase 7: calculators over `ocalc-finance`) ------------------

    /// Recompute every finance calculator from the current fields.
    pub fn refresh_finance(&mut self) {
        let f = self.finance.clone();
        match commands::finance_compound(&f.compound_p, &f.compound_r, &f.compound_n) {
            Ok(v) => {
                self.finance.compound_result = Some(v);
                self.finance.compound_error = None;
            }
            Err(msg) => {
                self.finance.compound_result = None;
                self.finance.compound_error = Some(msg);
            }
        }
        match commands::finance_effective_rate(&f.compound_nominal, &f.compound_m) {
            Ok(v) => {
                self.finance.compound_effective = Some(v);
                self.finance.compound_effective_error = None;
            }
            Err(msg) => {
                self.finance.compound_effective = None;
                self.finance.compound_effective_error = Some(msg);
            }
        }
        match commands::finance_loan(&f.loan_p, &f.loan_apr, &f.loan_n) {
            Ok((pmt, total, interest)) => {
                self.finance.loan_payment = Some(pmt);
                self.finance.loan_total = Some(total);
                self.finance.loan_interest = Some(interest);
                self.finance.loan_error = None;
            }
            Err(msg) => {
                self.finance.loan_payment = None;
                self.finance.loan_total = None;
                self.finance.loan_interest = None;
                self.finance.loan_error = Some(msg);
            }
        }
        match commands::finance_growth(&f.growth_start, &f.growth_rate, &f.growth_n) {
            Ok(v) => {
                self.finance.growth_result = Some(v);
                self.finance.growth_error = None;
            }
            Err(msg) => {
                self.finance.growth_result = None;
                self.finance.growth_error = Some(msg);
            }
        }
        match commands::finance_growth_rate(&f.rate_start, &f.rate_end, &f.rate_n) {
            Ok(v) => {
                self.finance.rate_result = Some(v);
                self.finance.rate_error = None;
            }
            Err(msg) => {
                self.finance.rate_result = None;
                self.finance.rate_error = Some(msg);
            }
        }
        match commands::finance_percent(&f.percent_base, &f.percent_pct) {
            Ok(v) => {
                self.finance.percent_result = Some(v);
                self.finance.percent_error = None;
            }
            Err(msg) => {
                self.finance.percent_result = None;
                self.finance.percent_error = Some(msg);
            }
        }
        match commands::finance_percent_change(&f.change_old, &f.change_new) {
            Ok(v) => {
                self.finance.change_result = Some(v);
                self.finance.change_error = None;
            }
            Err(msg) => {
                self.finance.change_result = None;
                self.finance.change_error = Some(msg);
            }
        }
    }

    /// Format a currency-style value at the current display precision.
    #[must_use]
    pub fn fmt_money(&self, v: f64) -> String {
        self.format_result(&Value::Float(v))
    }

    /// Format a fraction as a human percentage (`0.05 → 5 %`-style number).
    #[must_use]
    pub fn fmt_pct(&self, frac: f64) -> String {
        self.format_result(&Value::Float(frac * 100.0))
    }

    /// Format an arbitrary [`Value`] for view readouts (physics constants,
    /// chemistry masses) with the active precision + number format.
    #[must_use]
    pub fn fmt_value(&self, value: &Value) -> String {
        self.format_result(value)
    }

    // -- Favorites (starred calculations, persisted via settings) -----------

    /// True when `expression` is starred.
    #[must_use]
    pub fn is_favorited(&self, expression: &str) -> bool {
        self.favorites.iter().any(|f| f.expression == expression)
    }

    /// Mirror favorites into settings so autosave/manual save persist them.
    fn sync_favorites(&mut self) {
        self.settings.favorites = self.favorites.clone();
    }

    /// Star the current input + display (evaluates first when needed).
    pub fn toggle_favorite_current(&mut self) {
        if self.last_result.is_none() && !self.input.trim().is_empty() {
            self.submit();
        }
        let expression = self.input.trim().to_owned();
        if expression.is_empty() {
            self.notify("Nothing to star".to_owned(), true);
            return;
        }
        let result = self.display.clone();
        self.toggle_favorite(expression, result);
    }

    /// Star or unstar a history entry by id.
    pub fn toggle_favorite_entry(&mut self, id: u64) {
        match self.history.all().iter().find(|e| e.id == id) {
            Some(entry) => {
                let (expression, result) = (entry.expression.clone(), entry.result.clone());
                self.toggle_favorite(expression, result);
            }
            None => self.notify(format!("No history entry #{id}"), true),
        }
    }

    /// Star `expression`, or unstar it when already starred.
    pub fn toggle_favorite(&mut self, expression: String, result: String) {
        if self.favorites.iter().any(|f| f.expression == expression) {
            self.favorites.retain(|f| f.expression != expression);
            self.sync_favorites();
            self.notify("Removed from favorites".to_owned(), false);
            return;
        }
        let id = self.next_fav_id;
        self.next_fav_id += 1;
        self.favorites.push(FavoriteEntry {
            id,
            expression,
            result,
        });
        while self.favorites.len() > MAX_FAVORITES {
            self.favorites.remove(0);
        }
        self.sync_favorites();
        self.notify("Added to favorites".to_owned(), false);
    }

    /// Load a favorite's expression back into the input.
    pub fn reuse_favorite(&mut self, id: u64) {
        if let Some(entry) = self.favorites.iter().find(|f| f.id == id) {
            self.input = entry.expression.clone();
            self.error = None;
        }
    }

    /// Delete one favorite (others keep their ids).
    pub fn delete_favorite(&mut self, id: u64) {
        let before = self.favorites.len();
        self.favorites.retain(|f| f.id != id);
        if self.favorites.len() != before {
            self.sync_favorites();
            self.notify(format!("Removed favorite #{id}"), false);
        }
    }

    /// Clear all favorites.
    pub fn clear_favorites(&mut self) {
        self.favorites.clear();
        self.sync_favorites();
        self.notify("Favorites cleared".to_owned(), false);
    }

    // -- Graph hover (coordinate readout) ------------------------------------

    /// Record hovered data coordinates from the canvas.
    pub fn graph_set_hover(&mut self, x: f64, y: f64) {
        self.graph.hover = Some((x, y));
    }

    /// Clear the hover readout (data changed or pointer left).
    pub fn graph_clear_hover(&mut self) {
        self.graph.hover = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_ok_pushes_history() {
        let mut s = AppState::new();
        s.input = "1 + 2".to_owned();
        s.submit();
        assert_eq!(s.display, "3");
        assert!(s.error.is_none());
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.status.eval_count, 1);
        assert!(s.status.last_duration_us.is_some());
    }

    #[test]
    fn submit_err_sets_error_and_toast() {
        let mut s = AppState::new();
        s.input = "1 + ".to_owned();
        s.submit();
        assert!(s.last_result.is_none());
        assert!(s.error.is_some());
        assert!(s.history.is_empty());
        let toast = s.toast.clone().unwrap();
        assert!(toast.is_error);
        s.dismiss_toast();
        assert!(s.toast.is_none());
    }

    #[test]
    fn negate_toggles() {
        let mut s = AppState::new();
        s.negate();
        assert_eq!(s.input, "-");
        s.negate();
        assert!(s.input.is_empty());
        s.input = "5".to_owned();
        s.negate();
        assert_eq!(s.input, "-5");
    }

    #[test]
    fn clear_entry_keeps_result() {
        let mut s = AppState::new();
        s.input = "1+1".to_owned();
        s.submit();
        s.clear_entry();
        assert!(s.input.is_empty());
        assert_eq!(s.display, "2");
    }

    #[test]
    fn precision_reformats() {
        let mut s = AppState::new();
        s.input = "2.567".to_owned();
        s.submit();
        s.set_precision(2);
        assert_eq!(s.display, "2.57");
        s.set_precision(0);
        assert_eq!(s.display, "3");
    }

    #[test]
    fn angle_switches_engine() {
        let mut s = AppState::new();
        assert_eq!(s.angle, AngleUnit::Radians);
        s.set_angle_unit(AngleUnit::Degrees);
        assert_eq!(s.angle, AngleUnit::Degrees);
        assert_eq!(s.settings.angle, AngleUnit::Degrees);
        s.input = "sin(90)".to_owned();
        s.submit();
        assert!((s.last_result.as_ref().unwrap().to_f64() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn grad_mode_evaluates() {
        let mut s = AppState::new();
        s.set_angle_unit(AngleUnit::Gradians);
        assert_eq!(s.engine.angle_mode(), AngleMode::Radians);
        s.input = "sin(100)".to_owned();
        s.submit();
        assert!((s.last_result.as_ref().unwrap().to_f64() - 1.0).abs() < 1e-9);
        // History keeps the user's original text.
        assert_eq!(s.history.all()[0].expression, "sin(100)");
    }

    #[test]
    fn history_search_reuse_delete() {
        let mut s = AppState::new();
        s.input = "1+1".to_owned();
        s.submit();
        s.input = "sin(0)".to_owned();
        s.submit();
        s.history_search = "sin".to_owned();
        assert_eq!(s.filtered_history().len(), 1);
        let first_id = s.history.all()[0].id;
        s.reuse_history(first_id);
        assert_eq!(s.input, "1+1");
        let second_id = s.history.all()[1].id;
        s.delete_history(second_id);
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.history.all()[0].expression, "1+1");
        s.clear_history();
        assert!(s.history.is_empty());
    }

    #[test]
    fn history_capacity_enforced() {
        let mut s = AppState::new();
        for i in 0..12 {
            s.input = format!("{i}+0");
            s.submit();
        }
        assert_eq!(s.history.len(), 12);
        // Lowering the cap at runtime drops the oldest, preserving ids.
        s.settings.history_capacity = 10;
        s.apply_settings();
        assert_eq!(s.history.len(), 10);
        assert_eq!(s.history.all()[0].expression, "2+0");
        let ids: std::collections::HashSet<u64> = s.history.all().iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 10);
        // New pushes continue the monotonic id sequence.
        s.input = "99+0".to_owned();
        s.submit();
        assert_eq!(s.history.len(), 10);
        assert_eq!(s.history.all()[9].expression, "99+0");
    }

    #[test]
    fn memory_flow() {
        let mut s = AppState::new();
        assert!(s.memory.is_none());
        s.memory_recall();
        assert!(s.toast.clone().is_some_and(|t| t.is_error));
        s.input = "10".to_owned();
        s.submit();
        s.memory_store();
        assert!(s.memory.is_some());
        s.input = "5".to_owned();
        s.submit();
        s.memory_combine(true);
        assert_eq!(s.memory.clone().unwrap().to_string(), "15");
        s.memory_combine(false);
        assert_eq!(s.memory.clone().unwrap().to_string(), "10");
        s.memory_recall();
        assert_eq!(s.input, "10");
        s.memory_clear();
        assert!(s.memory.is_none());
    }

    #[test]
    fn graph_and_converter_refresh() {
        let mut s = AppState::new();
        assert!(!s.graph.funcs[0].samples.is_empty());
        s.graph.funcs[0].expr = "???".to_owned();
        s.refresh_graph();
        assert!(s.graph.funcs[0].error.is_some());
        s.converter.value_text = "1".to_owned();
        s.converter.from = "km".to_owned();
        s.converter.to = "m".to_owned();
        s.refresh_converter();
        assert_eq!(s.converter.result, Some(1000.0));
    }

    #[test]
    fn graph_multi_zoom_pan() {
        let mut s = AppState::new();
        assert_eq!(s.graph.funcs.len(), 2);
        s.graph_add_func();
        assert_eq!(s.graph.funcs.len(), 3);
        let id = s.graph.funcs[2].id;
        s.graph_toggle_func(id);
        assert!(!s.graph.funcs[2].visible);
        s.graph_remove_func(id);
        assert_eq!(s.graph.funcs.len(), 2);
        // Zoom/pan keep a valid ordered range.
        s.graph.zoom_in();
        s.graph.pan_by(0.1);
        let (from, to) = s.graph.effective_range();
        assert!(from < to && from.is_finite() && to.is_finite());
        s.graph.reset_view();
        assert_eq!((s.graph.zoom, s.graph.pan), (1.0, 0.0));
    }

    #[test]
    fn converter_category_defaults() {
        let mut s = AppState::new();
        s.set_converter_category(UnitCategoryId::Speed);
        assert_eq!(s.converter.category, UnitCategoryId::Speed);
        assert!(s.converter.result.is_some());
        s.set_converter_category(UnitCategoryId::Temperature);
        assert!(s.converter.result.is_some());
    }

    #[test]
    fn toggle_bit_rebuilds_through_engine() {
        let mut s = AppState::new();
        s.toggle_bit(0);
        assert!(s.toast.clone().is_some_and(|t| t.is_error));
        s.input = "8".to_owned(); // 0b1000
        s.submit();
        s.toggle_bit(0); // → 9
        assert_eq!(s.display, "9");
        s.toggle_bit(0); // → 8
        assert_eq!(s.display, "8");
    }

    #[test]
    fn settings_roundtrip_and_clamp() {
        let dir = std::env::temp_dir().join("ocalc-desktop-test-settings.json");
        let st = Settings {
            precision: 99,
            font_scale: 5.0,
            history_capacity: 1,
            startup_mode: CalcMode::Graph,
            ..Settings::default()
        };
        let clamped = st.clamped();
        assert_eq!(clamped.precision, commands::MAX_PRECISION);
        assert_eq!(clamped.font_scale, 1.5);
        assert_eq!(clamped.history_capacity, 10);
        clamped.save(&dir).unwrap();
        let back = Settings::load(&dir);
        assert_eq!(back.startup_mode, CalcMode::Graph);
        let _ = std::fs::remove_file(&dir);
        // With-settings constructor honors startup mode.
        let s = AppState::with_settings(back);
        assert_eq!(s.mode, CalcMode::Graph);
    }

    #[test]
    fn scientific_modules_discovered_by_default() {
        let s = AppState::new();
        for id in crate::modules::SCIENTIFIC_MODULE_IDS {
            assert!(
                s.engine.module_ids().contains(&id.to_owned()),
                "{:?}",
                s.engine.module_ids()
            );
            assert!(s.is_module_enabled(id));
        }
        assert_eq!(s.scientific_modules().len(), 4);
        // Scientific kernels evaluate through the engine.
        let mut s = s;
        assert!((s.engine.eval("force(2, 3)").unwrap().to_f64() - 6.0).abs() < 1e-12);
        assert!((s.engine.eval("compound(1000, 0.05, 1)").unwrap().to_f64() - 1050.0).abs() < 1e-9);
    }

    #[test]
    fn module_enable_disable_roundtrip() {
        let mut s = AppState::new();
        assert!(s.is_module_enabled("physics"));
        s.set_module_enabled("physics", false);
        assert!(!s.is_module_enabled("physics"));
        assert!(!s.engine.module_ids().contains(&"physics".to_owned()));
        assert!(s.engine.eval("force(2, 3)").is_err());
        s.set_module_enabled("physics", true);
        assert!(s.is_module_enabled("physics"));
        assert!((s.engine.eval("force(2, 3)").unwrap().to_f64() - 6.0).abs() < 1e-12);
        // Unknown ids are rejected without state change.
        s.set_module_enabled("nosuch", false);
        assert!(s.toast.clone().is_some_and(|t| t.is_error));
        // Toggle helper flips.
        s.toggle_module("finance");
        assert!(!s.is_module_enabled("finance"));
        s.toggle_module("finance");
        assert!(s.is_module_enabled("finance"));
    }

    #[test]
    fn function_search_filters() {
        let mut s = AppState::new();
        s.function_search = String::new();
        assert!(s.filtered_functions().contains(&"sin".to_owned()));
        assert!(s.filtered_functions().contains(&"force".to_owned()));
        s.function_search = "force".to_owned();
        assert_eq!(s.filtered_functions(), vec!["force".to_owned()]);
        s.function_search = "MOLAR".to_owned();
        assert_eq!(s.filtered_functions(), vec!["molar_mass".to_owned()]);
        s.function_search = "nosuchfunction_xyz".to_owned();
        assert!(s.filtered_functions().is_empty());
    }

    #[test]
    fn scientific_modes_navigate() {
        let mut s = AppState::new();
        for mode in [
            CalcMode::Cas,
            CalcMode::Physics,
            CalcMode::Chemistry,
            CalcMode::Finance,
        ] {
            s.mode = mode;
            assert!(matches!(
                s.mode,
                CalcMode::Cas | CalcMode::Physics | CalcMode::Chemistry | CalcMode::Finance
            ));
        }
        assert!(!matches!(
            CalcMode::Standard,
            CalcMode::Cas | CalcMode::Physics | CalcMode::Chemistry | CalcMode::Finance
        ));
        assert_eq!(CalcMode::MAIN.len(), 5);
        assert_eq!(CalcMode::SCIENTIFIC.len(), 4);
        assert_eq!(CalcMode::ALL.len(), 9);
        // Shortcut digits stay unique across all nine modes.
        let mut digits: Vec<char> = CalcMode::ALL.iter().map(|m| m.shortcut_digit()).collect();
        digits.sort();
        digits.dedup();
        assert_eq!(digits.len(), 9);
    }

    #[test]
    fn cas_state_flows() {
        let mut s = AppState::new();
        // Simplify pushes history.
        s.cas.input = "x + x".to_owned();
        s.cas_simplify();
        assert_eq!(s.cas.result.as_deref(), Some("2*x"));
        assert!(s.cas.error.is_none());
        assert_eq!(s.cas.history.len(), 1);
        assert_eq!(s.cas.history[0].op, "simplify");
        // Derivative honors the variable field.
        s.cas.input = "x^2".to_owned();
        s.cas.var = "x".to_owned();
        s.cas_differentiate();
        assert_eq!(s.cas.result.as_deref(), Some("2*x"));
        // Expand / factor / integrate.
        s.cas.input = "(x + 1)^2".to_owned();
        s.cas_expand();
        assert_eq!(s.cas.result.as_deref(), Some("x^2 + 2*x + 1"));
        s.cas.input = "2*x + 4".to_owned();
        s.cas_factor();
        assert_eq!(s.cas.result.as_deref(), Some("2*(x + 2)"));
        s.cas.input = "x".to_owned();
        s.cas_integrate();
        assert_eq!(s.cas.result.as_deref(), Some("0.5*x^2"));
        // Solvers use their field groups.
        s.cas.lin_a = "2".to_owned();
        s.cas.lin_b = "-4".to_owned();
        s.cas_solve_linear();
        assert_eq!(s.cas.result.as_deref(), Some("x = 2"));
        s.cas.quad_a = "1".to_owned();
        s.cas.quad_b = "0".to_owned();
        s.cas.quad_c = "-4".to_owned();
        s.cas_solve_quadratic();
        assert_eq!(s.cas.result.as_deref(), Some("x = -2 or 2"));
        // Errors surface without history growth.
        let len = s.cas.history.len();
        s.cas.input = "1 + ".to_owned();
        s.cas_simplify();
        assert!(s.cas.result.is_none());
        assert!(s.cas.error.is_some());
        assert_eq!(s.cas.history.len(), len);
        // Reuse + clear.
        let id = s.cas.history[0].id;
        s.cas_reuse(id);
        assert_eq!(s.cas.input, "x + x");
        s.cas_clear_history();
        assert!(s.cas.history.is_empty());
    }

    #[test]
    fn physics_state_flows() {
        let mut s = AppState::new();
        // Defaults compute on startup (force 2·3).
        assert_eq!(s.physics.result.as_deref(), Some("6"));
        assert_eq!(s.physics.unit, "N");
        // Formula switch resets inputs and recomputes.
        s.set_physics_formula(PhysicsFormula::Velocity);
        assert_eq!(s.physics.formula, PhysicsFormula::Velocity);
        s.physics.a_text = "100".to_owned();
        s.physics.b_text = "10".to_owned();
        s.refresh_physics();
        assert_eq!(s.physics.result.as_deref(), Some("10"));
        assert_eq!(s.physics.unit, "m/s");
        // Single-input formula hides the second field.
        s.set_physics_formula(PhysicsFormula::Weight);
        assert_eq!(s.physics.formula.b_label(), None);
        s.physics.a_text = "10".to_owned();
        s.refresh_physics();
        assert!(s.physics.result.is_some());
        assert_eq!(s.physics.unit, "N");
        // Domain errors surface.
        s.set_physics_formula(PhysicsFormula::Current);
        s.physics.a_text = "1".to_owned();
        s.physics.b_text = "0".to_owned();
        s.refresh_physics();
        assert!(s.physics.result.is_none());
        assert!(s.physics.error.is_some());
        // Selector metadata covers every engine function id.
        for f in PhysicsFormula::ALL {
            assert!(!f.id().is_empty());
            assert!(!f.label().is_empty());
            assert!(!f.a_label().is_empty());
        }
    }

    #[test]
    fn chemistry_state_flows() {
        let mut s = AppState::new();
        // Defaults analyze H2O on startup.
        assert!(s
            .chemistry
            .result_mass
            .is_some_and(|m| (m - 18.015).abs() < 1e-9));
        assert_eq!(s.chemistry.breakdown.len(), 2);
        assert!(s.chemistry.error.is_none());
        // Formula edits re-analyze.
        s.chemistry.formula = "CO2".to_owned();
        s.refresh_chemistry();
        assert!(s
            .chemistry
            .result_mass
            .is_some_and(|m| (m - 44.009).abs() < 1e-9));
        // Bad input surfaces errors and clears the breakdown.
        s.chemistry.formula = "Xx2".to_owned();
        s.refresh_chemistry();
        assert!(s.chemistry.result_mass.is_none());
        assert!(s.chemistry.breakdown.is_empty());
        assert!(s.chemistry.error.is_some());
        // Element panel + table search.
        assert_eq!(s.chemistry_selected_card().unwrap().symbol, "O");
        s.chemistry.selected = "C".to_owned();
        assert_eq!(s.chemistry_selected_card().unwrap().name, "Carbon");
        s.chemistry.selected = "Xx".to_owned();
        assert!(s.chemistry_selected_card().is_none());
        s.chemistry.table_search = "oxy".to_owned();
        let hits = s.chemistry_filtered_table();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].symbol, "O");
        s.chemistry.table_search = String::new();
        assert_eq!(s.chemistry_filtered_table().len(), 118);
    }

    #[test]
    fn finance_state_flows() {
        let mut s = AppState::new();
        // Defaults compute on startup (docs examples).
        assert!(s
            .finance
            .compound_result
            .is_some_and(|v| (v - 1628.894626777442).abs() < 1e-6));
        assert!(s.finance.compound_effective.is_some());
        assert!(s
            .finance
            .loan_payment
            .is_some_and(|v| (v - 860.66).abs() < 0.01));
        assert!(s.finance.loan_total.is_some());
        assert!(s.finance.loan_interest.is_some());
        assert!(s
            .finance
            .growth_result
            .is_some_and(|v| (v - 121.0).abs() < 1e-9));
        assert!(s
            .finance
            .rate_result
            .is_some_and(|v| (v - 0.1).abs() < 1e-12));
        assert_eq!(s.finance.percent_result, Some(30.0));
        assert_eq!(s.finance.change_result, Some(20.0));
        assert!(s.finance.compound_error.is_none());
        assert!(s.finance.loan_error.is_none());
        // Edits recompute; bad input surfaces per-tool errors.
        s.finance.compound_p = "1000".to_owned();
        s.finance.compound_r = "5".to_owned();
        s.finance.compound_n = "1".to_owned();
        s.refresh_finance();
        assert!(s
            .finance
            .compound_result
            .is_some_and(|v| (v - 1050.0).abs() < 1e-9));
        s.finance.loan_n = "0".to_owned();
        s.refresh_finance();
        assert!(s.finance.loan_payment.is_none());
        assert!(s.finance.loan_error.is_some());
        // Display helpers honor precision.
        assert_eq!(s.fmt_money(2.5), "2.5");
        assert_eq!(s.fmt_pct(0.05), "5");
    }
}
