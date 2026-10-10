//! [`OcalcApp`]: Iced update/view/subscription wiring.
//!
//! Update flow per message: `Message → AppState method → commands.rs →
//! stable Engine API → AppState updated`. Views read state only. Settings
//! persistence (filesystem) happens in [`OcalcApp::update`] — never in the
//! pure [`OcalcApp::update_inner`] core, which stays headless-testable.

use iced::{Element, Subscription, Task};

use crate::commands;
use crate::messages::Message;
use crate::state::{AppState, CalcMode, Settings};
use crate::views;

/// Root application state (Iced `State`).
#[derive(Debug)]
pub struct OcalcApp {
    /// All UI + engine state.
    pub state: AppState,
}

impl Default for OcalcApp {
    fn default() -> Self {
        Self::new()
    }
}

impl OcalcApp {
    /// Create with persisted settings applied (best-effort load).
    ///
    /// Production boot: XDG dirs are created, legacy files migrated, startup
    /// is logged and the module inventory is reported to the log. All
    /// best-effort — a read-only HOME must never prevent startup.
    #[must_use]
    pub fn new() -> Self {
        let _ = ocalc_history::ensure_dirs();
        let migrated = ocalc_history::migrate_legacy();
        ocalc_history::log_startup("ocalc-desktop", env!("CARGO_PKG_VERSION"));
        if migrated.settings_migrated || migrated.history_migrated {
            ocalc_history::log_event(
                "config",
                &format!(
                    "legacy migration (settings: {}, history: {})",
                    migrated.settings_migrated, migrated.history_migrated
                ),
            );
        }
        let settings = Settings::load(&commands::default_settings_path());
        let app = Self {
            state: AppState::with_settings(settings),
        };
        let (loaded, failures) = crate::modules::module_load_report(&app.state.engine);
        ocalc_history::log_event("module", &format!("loaded [{}]", loaded.join(", ")));
        for f in &failures {
            ocalc_history::log_event("module", f);
            eprintln!("warning: {f}");
        }
        app
    }

    /// Iced update: pure transition, then best-effort settings persistence
    /// for settings-affecting messages.
    ///
    /// Two messages also produce runtime tasks: [`Message::CopyResult`]
    /// writes to the clipboard, and [`Message::ModeChanged`] moves keyboard
    /// focus to the new mode's primary input.
    pub fn update(state: &mut Self, message: Message) -> Task<Message> {
        if matches!(message, Message::CopyResult) {
            Self::update_inner(&mut state.state, Message::CopyResult);
            let text = state.state.display.clone();
            if text.is_empty() {
                return Task::none();
            }
            return iced::clipboard::write(text);
        }
        let focus = match &message {
            Message::ModeChanged(mode) => focus_task_for(*mode),
            _ => Task::none(),
        };
        let persist = message.affects_settings();
        Self::update_inner(&mut state.state, message);
        if persist {
            if let Err(e) = state
                .state
                .settings
                .save(&commands::default_settings_path())
            {
                let msg = format!("settings autosave failed: {e}");
                eprintln!("warning: {msg}");
                ocalc_history::log_event("config", &msg);
            }
        }
        focus
    }

    /// Pure transition (unit-testable without `Task` or filesystem).
    pub fn update_inner(state: &mut AppState, message: Message) {
        match message {
            Message::InputChanged(s) => {
                state.input = s;
            }
            Message::AppendInput(s) => {
                state.append(&s);
            }
            Message::Evaluate => state.submit(),
            Message::Clear => state.clear_all(),
            Message::ClearEntry => state.clear_entry(),
            Message::Backspace => state.backspace(),
            Message::Negate => state.negate(),

            Message::MemoryClear => state.memory_clear(),
            Message::MemoryRecall => state.memory_recall(),
            Message::MemoryStore => state.memory_store(),
            Message::MemoryAdd => state.memory_combine(true),
            Message::MemorySub => state.memory_combine(false),

            Message::ModeChanged(mode) => {
                state.mode = mode;
                if mode == CalcMode::Graph {
                    state.refresh_graph();
                }
            }
            Message::AngleUnitChanged(unit) => state.set_angle_unit(unit),
            Message::PrecisionChanged(p) => state.set_precision(p),
            Message::NumberFormatChanged(format) => state.set_number_format(format),
            Message::ThemeChanged(mode) => state.set_theme(mode),
            Message::AccentChanged(idx) => {
                state.settings.accent_idx = idx;
                state.apply_settings();
            }
            Message::FontScaleChanged(scale) => {
                state.settings.font_scale = scale;
                state.apply_settings();
            }
            Message::HistoryCapacityChanged(cap) => {
                state.settings.history_capacity = cap.clamp(10, 10_000);
                state.apply_settings();
            }
            Message::HistoryAutoTrimChanged(auto) => {
                state.settings.history_auto_trim = auto;
                state.apply_settings();
            }
            Message::StartupModeChanged(mode) => {
                state.settings.startup_mode = mode;
            }
            Message::ToggleHistory => {
                state.settings.show_history = !state.settings.show_history;
            }
            Message::ToggleSettings => {
                state.settings.show_settings = !state.settings.show_settings;
            }
            Message::SettingsSave => {
                match state.settings.save(&commands::default_settings_path()) {
                    Ok(()) => state.notify("Settings saved".to_owned(), false),
                    Err(msg) => state.notify(format!("Save failed: {msg}"), true),
                }
            }
            Message::DismissToast => state.dismiss_toast(),

            Message::HistorySearchChanged(q) => {
                state.history_search = q;
            }
            Message::HistoryReuse(id) => state.reuse_history(id),
            Message::HistoryDelete(id) => state.delete_history(id),
            Message::HistoryClear => state.clear_history(),
            Message::CopyResult => {
                if state.display.is_empty() {
                    state.notify("Nothing to copy".to_owned(), true);
                } else {
                    state.notify("Result copied".to_owned(), false);
                }
            }
            Message::FavoriteToggleCurrent => state.toggle_favorite_current(),
            Message::FavoriteToggleEntry(id) => state.toggle_favorite_entry(id),
            Message::FavoriteReuse(id) => state.reuse_favorite(id),
            Message::FavoriteDelete(id) => state.delete_favorite(id),
            Message::FavoritesClear => state.clear_favorites(),

            Message::GraphFuncChanged(id, s) => {
                if let Some(f) = state.graph.funcs.iter_mut().find(|f| f.id == id) {
                    f.expr = s;
                }
                state.refresh_graph();
            }
            Message::GraphAddFunc => state.graph_add_func(),
            Message::GraphRemoveFunc(id) => state.graph_remove_func(id),
            Message::GraphToggleFunc(id) => state.graph_toggle_func(id),
            Message::GraphBoundChanged(which, s) => {
                if which == 0 {
                    state.graph.from_text = s;
                } else {
                    state.graph.to_text = s;
                }
                state.graph.reset_view();
                state.refresh_graph();
            }
            Message::GraphPointsChanged(n) => {
                state.graph.points = n.clamp(2, 10_000);
                state.refresh_graph();
            }
            Message::GraphZoomIn => {
                state.graph.zoom_in();
                state.refresh_graph();
            }
            Message::GraphZoomOut => {
                state.graph.zoom_out();
                state.refresh_graph();
            }
            Message::GraphPanLeft => {
                state.graph.pan_by(-0.1);
                state.refresh_graph();
            }
            Message::GraphPanRight => {
                state.graph.pan_by(0.1);
                state.refresh_graph();
            }
            Message::GraphResetView => {
                state.graph.reset_view();
                state.refresh_graph();
            }
            Message::GraphToggleGrid => {
                state.graph.show_grid = !state.graph.show_grid;
            }
            Message::GraphHover(x, y) => state.graph_set_hover(x, y),

            Message::ConverterCategoryChanged(cat) => state.set_converter_category(cat),
            Message::ConverterValueChanged(s) => {
                state.converter.value_text = s;
                state.refresh_converter();
            }
            Message::ConverterUnitChanged(which, s) => {
                if which == 0 {
                    state.converter.from = s;
                } else {
                    state.converter.to = s;
                }
                state.refresh_converter();
            }
            Message::ConverterTimeUnitChanged(which, s) => {
                if which == 0 {
                    state.converter.speed_from_time = s;
                } else {
                    state.converter.speed_to_time = s;
                }
                state.refresh_converter();
            }
            Message::ConverterSwap => {
                std::mem::swap(&mut state.converter.from, &mut state.converter.to);
                std::mem::swap(
                    &mut state.converter.speed_from_time,
                    &mut state.converter.speed_to_time,
                );
                state.refresh_converter();
            }

            Message::ProgrammerBaseChanged(base) => {
                state.programmer.base = base;
            }
            Message::ProgrammerToggleBits => {
                state.programmer.show_bits = !state.programmer.show_bits;
            }
            Message::ProgrammerSignedChanged(signed) => {
                state.programmer.signed = signed;
            }
            Message::ProgrammerToggleBit(i) => state.toggle_bit(i),

            Message::ModuleToggled(id) => state.toggle_module(&id),
            Message::FunctionSearchChanged(q) => {
                state.function_search = q;
            }

            Message::CasInputChanged(s) => {
                state.cas.input = s;
            }
            Message::CasVarChanged(s) => {
                state.cas.var = s;
            }
            Message::CasSimplify => state.cas_simplify(),
            Message::CasExpand => state.cas_expand(),
            Message::CasFactor => state.cas_factor(),
            Message::CasDifferentiate => state.cas_differentiate(),
            Message::CasIntegrate => state.cas_integrate(),
            Message::CasLinChanged(which, s) => {
                if which == 0 {
                    state.cas.lin_a = s;
                } else {
                    state.cas.lin_b = s;
                }
            }
            Message::CasQuadChanged(which, s) => {
                if which == 0 {
                    state.cas.quad_a = s;
                } else if which == 1 {
                    state.cas.quad_b = s;
                } else {
                    state.cas.quad_c = s;
                }
            }
            Message::CasSolveLinear => state.cas_solve_linear(),
            Message::CasSolveQuadratic => state.cas_solve_quadratic(),
            Message::CasHistoryClear => state.cas_clear_history(),
            Message::CasReuse(id) => state.cas_reuse(id),
            Message::CasToggleSolvers => {
                state.cas.show_solvers = !state.cas.show_solvers;
            }
            Message::CasToggleHistory => {
                state.cas.show_history = !state.cas.show_history;
            }

            Message::PhysicsFormulaChanged(f) => state.set_physics_formula(f),
            Message::PhysicsAChanged(s) => {
                state.physics.a_text = s;
                state.refresh_physics();
            }
            Message::PhysicsBChanged(s) => {
                state.physics.b_text = s;
                state.refresh_physics();
            }
            Message::PhysicsToggleConstants => {
                state.physics.show_constants = !state.physics.show_constants;
            }

            Message::ChemistryFormulaChanged(s) => {
                state.chemistry.formula = s;
                state.refresh_chemistry();
            }
            Message::ChemistrySelect(sym) => {
                state.chemistry.selected = sym;
            }
            Message::ChemistrySearchChanged(q) => {
                state.chemistry.table_search = q;
            }

            Message::FinanceCompoundChanged(which, s) => {
                if which == 0 {
                    state.finance.compound_p = s;
                } else if which == 1 {
                    state.finance.compound_r = s;
                } else {
                    state.finance.compound_n = s;
                }
                state.refresh_finance();
            }
            Message::FinanceNominalChanged(which, s) => {
                if which == 0 {
                    state.finance.compound_nominal = s;
                } else {
                    state.finance.compound_m = s;
                }
                state.refresh_finance();
            }
            Message::FinanceLoanChanged(which, s) => {
                if which == 0 {
                    state.finance.loan_p = s;
                } else if which == 1 {
                    state.finance.loan_apr = s;
                } else {
                    state.finance.loan_n = s;
                }
                state.refresh_finance();
            }
            Message::FinanceGrowthChanged(which, s) => {
                if which == 0 {
                    state.finance.growth_start = s;
                } else if which == 1 {
                    state.finance.growth_rate = s;
                } else {
                    state.finance.growth_n = s;
                }
                state.refresh_finance();
            }
            Message::FinanceRateChanged(which, s) => {
                if which == 0 {
                    state.finance.rate_start = s;
                } else if which == 1 {
                    state.finance.rate_end = s;
                } else {
                    state.finance.rate_n = s;
                }
                state.refresh_finance();
            }
            Message::FinancePercentChanged(which, s) => {
                if which == 0 {
                    state.finance.percent_base = s;
                } else {
                    state.finance.percent_pct = s;
                }
                state.refresh_finance();
            }
            Message::FinanceChangeChanged(which, s) => {
                if which == 0 {
                    state.finance.change_old = s;
                } else {
                    state.finance.change_new = s;
                }
                state.refresh_finance();
            }

            Message::KeyboardEvent(event) => handle_keyboard(state, event),

            Message::Nop => {}
        }
    }

    /// Iced view: header + toolbar + content + sidebars + status bar.
    pub fn view(state: &Self) -> Element<'_, Message> {
        views::shell::view(&state.state)
    }

    /// Keyboard subscription: control keys + `Ctrl+1…9` mode shortcuts.
    ///
    /// Printable characters are handled by the focused text input; the global
    /// subscription only handles control keys so keystrokes are never doubled.
    pub fn subscription(_state: &Self) -> Subscription<Message> {
        iced::keyboard::listen().map(Message::KeyboardEvent)
    }

    /// Active Iced theme from the ODS theme mode.
    pub fn theme(state: &Self) -> iced::Theme {
        state.state.theme.to_iced()
    }
}

/// Primary input id per mode for keyboard-first navigation.
///
/// Returns the `text_input` id that should receive focus when entering
/// `mode`, or `None` when the mode has no single primary input.
fn focus_id_for_mode(mode: CalcMode) -> Option<&'static str> {
    match mode {
        CalcMode::Standard | CalcMode::Scientific => Some("calc-input"),
        CalcMode::Converter => Some("conv-value"),
        CalcMode::Cas => Some("cas-input"),
        CalcMode::Physics => Some("physics-a"),
        CalcMode::Chemistry => Some("chem-formula"),
        CalcMode::Finance => Some("fin-compound-p"),
        CalcMode::Programmer | CalcMode::Graph => None,
    }
}

/// Focus task for a mode switch (no-op when the mode has no primary input).
fn focus_task_for(mode: CalcMode) -> Task<Message> {
    match focus_id_for_mode(mode) {
        Some(id) => iced::widget::operation::focus(id),
        None => Task::none(),
    }
}

/// Global keyboard handling: Enter/Esc + printable operator passthrough when
/// the input is unfocused + `Ctrl/Cmd+1…9` mode shortcuts.
///
/// Phase 8: mode shortcuts accept both `Ctrl` (Linux/Windows) and `Cmd`
/// (macOS) via [`Modifiers::CTRL`] | [`Modifiers::COMMAND`], so `Ctrl+1…9`
/// works natively on every desktop. `Ctrl`/`Cmd` with any other key is
/// swallowed (never appended to the expression).
fn handle_keyboard(state: &mut AppState, event: iced::keyboard::Event) {
    use iced::keyboard::{Event as KeyEvent, Key, Modifiers};
    let KeyEvent::KeyPressed { key, modifiers, .. } = event else {
        return;
    };
    // Mode shortcuts: Ctrl+1…9 on Linux/Windows, Cmd+1…9 on macOS.
    if modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::COMMAND) {
        if let Key::Character(c) = &key {
            let mode = CalcMode::ALL
                .iter()
                .find(|m| m.shortcut_digit().to_string() == c.as_str());
            if let Some(mode) = mode {
                state.mode = *mode;
                if *mode == CalcMode::Graph {
                    state.refresh_graph();
                }
                return;
            }
        }
        return;
    }
    match &key {
        Key::Named(iced::keyboard::key::Named::Enter) => state.submit(),
        Key::Named(iced::keyboard::key::Named::Escape) => state.clear_all(),
        // Editing keys apply when no text input has focus (focused inputs
        // consume them first, so keystrokes are never doubled).
        Key::Named(iced::keyboard::key::Named::Backspace) => state.backspace(),
        Key::Named(iced::keyboard::key::Named::Delete) => state.clear_entry(),
        Key::Character(c) => {
            let s = c.as_str();
            // Printable calculator keys (digits/operators) append when the
            // focused widget didn't consume them as text input.
            if matches!(
                s,
                "0" | "1"
                    | "2"
                    | "3"
                    | "4"
                    | "5"
                    | "6"
                    | "7"
                    | "8"
                    | "9"
                    | "+"
                    | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "^"
                    | "("
                    | ")"
                    | "."
                    | "!"
                    | "="
            ) {
                if s == "=" {
                    state.submit();
                } else {
                    state.append(s);
                }
            }
        }
        _ => {}
    }
}

impl Message {
    /// True when the message mutates persisted [`Settings`].
    fn affects_settings(&self) -> bool {
        matches!(
            self,
            Message::AngleUnitChanged(_)
                | Message::PrecisionChanged(_)
                | Message::NumberFormatChanged(_)
                | Message::ThemeChanged(_)
                | Message::AccentChanged(_)
                | Message::FontScaleChanged(_)
                | Message::HistoryCapacityChanged(_)
                | Message::HistoryAutoTrimChanged(_)
                | Message::StartupModeChanged(_)
                | Message::ToggleHistory
                | Message::ToggleSettings
                | Message::FavoriteToggleCurrent
                | Message::FavoriteToggleEntry(_)
                | Message::FavoriteDelete(_)
                | Message::FavoritesClear
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ods_theme::ThemeMode;
    use crate::state::{AngleUnit, NumBase, UnitCategoryId};

    fn app() -> OcalcApp {
        OcalcApp {
            state: AppState::new(),
        }
    }

    #[test]
    fn input_and_evaluate_flow() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("1+2".into()));
        assert_eq!(a.state.input, "1+2");
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        assert_eq!(a.state.display, "3");
        assert_eq!(a.state.history.len(), 1);
    }

    #[test]
    fn clear_entry_and_all() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("123".into()));
        OcalcApp::update_inner(&mut a.state, Message::Backspace);
        assert_eq!(a.state.input, "12");
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        OcalcApp::update_inner(&mut a.state, Message::ClearEntry);
        assert!(a.state.input.is_empty());
        assert!(!a.state.display.is_empty());
        OcalcApp::update_inner(&mut a.state, Message::Clear);
        assert!(a.state.display.is_empty());
    }

    #[test]
    fn mode_and_settings_messages() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::ModeChanged(CalcMode::Graph));
        assert_eq!(a.state.mode, CalcMode::Graph);
        OcalcApp::update_inner(&mut a.state, Message::AngleUnitChanged(AngleUnit::Gradians));
        assert_eq!(a.state.angle, AngleUnit::Gradians);
        OcalcApp::update_inner(&mut a.state, Message::PrecisionChanged(2));
        assert_eq!(a.state.precision, 2);
        OcalcApp::update_inner(&mut a.state, Message::ThemeChanged(ThemeMode::Light));
        assert_eq!(a.state.theme.mode, ThemeMode::Light);
        OcalcApp::update_inner(&mut a.state, Message::AccentChanged(Some(1)));
        assert_eq!(a.state.settings.accent_idx, Some(1));
        OcalcApp::update_inner(&mut a.state, Message::FontScaleChanged(1.25));
        assert_eq!(a.state.theme.font_scale, 1.25);
        OcalcApp::update_inner(&mut a.state, Message::ToggleHistory);
        assert!(!a.state.settings.show_history);
        OcalcApp::update_inner(&mut a.state, Message::ToggleSettings);
        assert!(a.state.settings.show_settings);
        OcalcApp::update_inner(&mut a.state, Message::DismissToast);
        assert!(a.state.toast.is_none());
    }

    #[test]
    fn memory_messages() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("7".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        OcalcApp::update_inner(&mut a.state, Message::MemoryStore);
        assert!(a.state.memory.is_some());
        OcalcApp::update_inner(&mut a.state, Message::MemoryAdd);
        assert_eq!(a.state.memory.clone().unwrap().to_string(), "14");
        OcalcApp::update_inner(&mut a.state, Message::MemorySub);
        assert_eq!(a.state.memory.clone().unwrap().to_string(), "7");
        OcalcApp::update_inner(&mut a.state, Message::MemoryRecall);
        assert_eq!(a.state.input, "7");
        OcalcApp::update_inner(&mut a.state, Message::MemoryClear);
        assert!(a.state.memory.is_none());
    }

    #[test]
    fn history_messages() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("1+1".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("+1".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        assert_eq!(a.state.history.len(), 2);
        OcalcApp::update_inner(&mut a.state, Message::HistorySearchChanged("3".into()));
        assert_eq!(a.state.filtered_history().len(), 1);
        let first = a.state.history.all()[0].id;
        let second = a.state.history.all()[1].id;
        OcalcApp::update_inner(&mut a.state, Message::HistoryDelete(first));
        assert_eq!(a.state.history.len(), 1);
        assert_eq!(a.state.history.all()[0].id, second);
        OcalcApp::update_inner(&mut a.state, Message::InputChanged(String::new()));
        OcalcApp::update_inner(&mut a.state, Message::HistoryReuse(second));
        assert_eq!(a.state.input, "1+1+1");
        OcalcApp::update_inner(&mut a.state, Message::HistoryClear);
        assert!(a.state.history.is_empty());
    }

    #[test]
    fn graph_messages() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::GraphAddFunc);
        assert_eq!(a.state.graph.funcs.len(), 3);
        let id = a.state.graph.funcs[2].id;
        OcalcApp::update_inner(&mut a.state, Message::GraphFuncChanged(id, "x".into()));
        assert_eq!(a.state.graph.funcs[2].expr, "x");
        OcalcApp::update_inner(&mut a.state, Message::GraphToggleFunc(id));
        assert!(!a.state.graph.funcs[2].visible);
        OcalcApp::update_inner(&mut a.state, Message::GraphZoomIn);
        assert!(a.state.graph.zoom > 1.0);
        OcalcApp::update_inner(&mut a.state, Message::GraphPanRight);
        assert!(a.state.graph.pan > 0.0);
        OcalcApp::update_inner(&mut a.state, Message::GraphResetView);
        assert_eq!((a.state.graph.zoom, a.state.graph.pan), (1.0, 0.0));
        OcalcApp::update_inner(&mut a.state, Message::GraphToggleGrid);
        assert!(!a.state.graph.show_grid);
        OcalcApp::update_inner(&mut a.state, Message::GraphRemoveFunc(id));
        assert_eq!(a.state.graph.funcs.len(), 2);
        OcalcApp::update_inner(&mut a.state, Message::GraphPointsChanged(101));
        assert_eq!(a.state.graph.points, 101);
    }

    #[test]
    fn converter_messages() {
        let mut a = app();
        OcalcApp::update_inner(
            &mut a.state,
            Message::ConverterCategoryChanged(UnitCategoryId::Speed),
        );
        assert_eq!(a.state.converter.category, UnitCategoryId::Speed);
        OcalcApp::update_inner(&mut a.state, Message::ConverterValueChanged("2".into()));
        assert_eq!(a.state.converter.value_text, "2");
        OcalcApp::update_inner(&mut a.state, Message::ConverterSwap);
        assert_eq!(a.state.converter.from, "m");
    }

    #[test]
    fn programmer_messages() {
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::ProgrammerBaseChanged(NumBase::Hex));
        assert_eq!(a.state.programmer.base, NumBase::Hex);
        OcalcApp::update_inner(&mut a.state, Message::ProgrammerToggleBits);
        assert!(!a.state.programmer.show_bits);
        OcalcApp::update_inner(&mut a.state, Message::ProgrammerSignedChanged(false));
        assert!(!a.state.programmer.signed);
    }

    #[test]
    fn scientific_module_messages() {
        let mut a = app();
        assert!(a.state.is_module_enabled("physics"));
        OcalcApp::update_inner(&mut a.state, Message::ModuleToggled("physics".into()));
        assert!(!a.state.is_module_enabled("physics"));
        OcalcApp::update_inner(&mut a.state, Message::ModuleToggled("physics".into()));
        assert!(a.state.is_module_enabled("physics"));
        OcalcApp::update_inner(&mut a.state, Message::FunctionSearchChanged("force".into()));
        assert_eq!(a.state.function_search, "force");
        assert_eq!(a.state.filtered_functions(), vec!["force".to_owned()]);
    }

    #[test]
    fn scientific_mode_messages() {
        use crate::state::PhysicsFormula;
        let mut a = app();
        // Mode navigation across the scientific category.
        for mode in [
            CalcMode::Cas,
            CalcMode::Physics,
            CalcMode::Chemistry,
            CalcMode::Finance,
        ] {
            OcalcApp::update_inner(&mut a.state, Message::ModeChanged(mode));
            assert_eq!(a.state.mode, mode);
        }
        // CAS operations route through state.
        OcalcApp::update_inner(&mut a.state, Message::CasInputChanged("x + x".into()));
        assert_eq!(a.state.cas.input, "x + x");
        OcalcApp::update_inner(&mut a.state, Message::CasSimplify);
        assert_eq!(a.state.cas.result.as_deref(), Some("2*x"));
        OcalcApp::update_inner(&mut a.state, Message::CasVarChanged("y".into()));
        assert_eq!(a.state.cas.var, "y");
        // Physics formula + inputs route and recompute.
        OcalcApp::update_inner(
            &mut a.state,
            Message::PhysicsFormulaChanged(PhysicsFormula::Velocity),
        );
        assert_eq!(a.state.physics.formula, PhysicsFormula::Velocity);
        OcalcApp::update_inner(&mut a.state, Message::PhysicsAChanged("100".into()));
        OcalcApp::update_inner(&mut a.state, Message::PhysicsBChanged("10".into()));
        assert_eq!(a.state.physics.result.as_deref(), Some("10"));
        assert_eq!(a.state.physics.unit, "m/s");
        // Chemistry + finance inputs route and recompute.
        OcalcApp::update_inner(&mut a.state, Message::ChemistryFormulaChanged("CO2".into()));
        assert!(a.state.chemistry.result_mass.is_some());
        OcalcApp::update_inner(&mut a.state, Message::ChemistrySelect("C".into()));
        assert_eq!(a.state.chemistry.selected, "C");
        OcalcApp::update_inner(
            &mut a.state,
            Message::FinanceCompoundChanged(0, "2000".into()),
        );
        assert_eq!(a.state.finance.compound_p, "2000");
        assert!(a.state.finance.compound_result.is_some());
    }

    /// Build a synthetic key-press for tests (fields beyond key are inert).
    fn key_press(key: iced::keyboard::Key) -> Message {
        use iced::keyboard::{Event as KeyEvent, Location, Modifiers};
        Message::KeyboardEvent(KeyEvent::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    fn key_press_ctrl(key: iced::keyboard::Key) -> Message {
        use iced::keyboard::{Event as KeyEvent, Location, Modifiers};
        Message::KeyboardEvent(KeyEvent::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: Location::Standard,
            modifiers: Modifiers::CTRL,
            text: None,
            repeat: false,
        })
    }

    fn key_press_cmd(key: iced::keyboard::Key) -> Message {
        use iced::keyboard::{Event as KeyEvent, Location, Modifiers};
        Message::KeyboardEvent(KeyEvent::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: Location::Standard,
            modifiers: Modifiers::COMMAND,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn keyboard_enter_and_escape() {
        use iced::keyboard::Key;
        let mut a = app();
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("2*3".into()));
        OcalcApp::update_inner(
            &mut a.state,
            key_press(Key::Named(iced::keyboard::key::Named::Enter)),
        );
        assert_eq!(a.state.display, "6");
        OcalcApp::update_inner(
            &mut a.state,
            key_press(Key::Named(iced::keyboard::key::Named::Escape)),
        );
        assert!(a.state.input.is_empty());
    }

    #[test]
    fn keyboard_mode_shortcuts() {
        use iced::keyboard::Key;
        let mut a = app();
        for (digit, mode) in [
            ("1", CalcMode::Standard),
            ("2", CalcMode::Scientific),
            ("3", CalcMode::Programmer),
            ("4", CalcMode::Converter),
            ("5", CalcMode::Graph),
            ("6", CalcMode::Cas),
            ("7", CalcMode::Physics),
            ("8", CalcMode::Chemistry),
            ("9", CalcMode::Finance),
        ] {
            OcalcApp::update_inner(&mut a.state, key_press_ctrl(Key::Character(digit.into())));
            assert_eq!(a.state.mode, mode, "Ctrl+{digit}");
        }
        // Plain digits append instead of switching modes.
        OcalcApp::update_inner(&mut a.state, key_press(Key::Character("7".into())));
        assert_eq!(a.state.input, "7");
    }

    // -- Phase 8: professional UX polish ----------------------------------

    #[test]
    fn phase8_navigation_all_modes_via_message() {
        // Smoother navigation: every mode reachable via Message, shortcut
        // digits unique, Graph refreshes on entry.
        let mut a = app();
        for mode in CalcMode::ALL {
            OcalcApp::update_inner(&mut a.state, Message::ModeChanged(mode));
            assert_eq!(a.state.mode, mode);
        }
        assert_eq!(CalcMode::ALL.len(), 9);
        assert_eq!(CalcMode::MAIN.len(), 5);
        assert_eq!(CalcMode::SCIENTIFIC.len(), 4);
        let mut digits: Vec<char> = CalcMode::ALL.iter().map(|m| m.shortcut_digit()).collect();
        digits.sort();
        digits.dedup();
        assert_eq!(digits.len(), 9);
        // Graph entry re-samples (no panic on empty/invalid).
        OcalcApp::update_inner(&mut a.state, Message::ModeChanged(CalcMode::Graph));
        assert!(!a.state.graph.funcs.is_empty());
    }

    #[test]
    fn phase8_shortcuts_ctrl_and_cmd_both_switch() {
        // Keyboard-first workflow: Ctrl (Linux/Win) and Cmd (macOS) both work.
        use iced::keyboard::Key;
        let pairs = [
            ("1", CalcMode::Standard),
            ("2", CalcMode::Scientific),
            ("3", CalcMode::Programmer),
            ("4", CalcMode::Converter),
            ("5", CalcMode::Graph),
            ("6", CalcMode::Cas),
            ("7", CalcMode::Physics),
            ("8", CalcMode::Chemistry),
            ("9", CalcMode::Finance),
        ];
        let mut a = app();
        for (digit, mode) in pairs {
            OcalcApp::update_inner(&mut a.state, key_press_cmd(Key::Character(digit.into())));
            assert_eq!(a.state.mode, mode, "Cmd+{digit}");
        }
        let mut b = app();
        for (digit, mode) in pairs {
            OcalcApp::update_inner(&mut b.state, key_press_ctrl(Key::Character(digit.into())));
            assert_eq!(b.state.mode, mode, "Ctrl+{digit}");
        }
        // Ctrl with a non-digit never touches the mode or input.
        let mut c = app();
        OcalcApp::update_inner(&mut c.state, key_press_ctrl(Key::Character("q".into())));
        assert_eq!(c.state.mode, CalcMode::Standard);
        assert!(c.state.input.is_empty());
    }

    #[test]
    fn phase8_shortcut_reference_covers_all_modes() {
        // Reference panel stays in sync with CalcMode::ALL.
        use crate::views::settings::SHORTCUTS;
        for mode in CalcMode::ALL {
            let want = format!("Ctrl+{}", mode.shortcut_digit());
            assert!(
                SHORTCUTS.iter().any(|(k, _)| *k == want),
                "shortcuts missing {want} for {}",
                mode.label()
            );
        }
        assert!(SHORTCUTS.len() >= 14);
    }

    #[test]
    fn phase8_settings_persistence_full_roundtrip() {
        use crate::ods_theme::ThemeMode;
        use crate::state::{AngleUnit, NumberFormat, Settings};
        let dir = std::env::temp_dir().join("ocalc-phase8-settings.json");
        let st = Settings {
            config_version: crate::state::SETTINGS_FORMAT_VERSION,
            precision: 4,
            angle: AngleUnit::Degrees,
            theme: ThemeMode::Light,
            accent_idx: Some(2),
            font_scale: 1.25,
            history_capacity: 123,
            startup_mode: CalcMode::Finance,
            show_history: false,
            show_settings: true,
            number_format: NumberFormat::Scientific,
            history_auto_trim: false,
            favorites: vec![crate::state::FavoriteEntry {
                id: 7,
                expression: "1+2".to_owned(),
                result: "3".to_owned(),
            }],
        };
        st.save(&dir).unwrap();
        let back = Settings::load(&dir);
        assert_eq!(back.precision, 4);
        assert_eq!(back.angle, AngleUnit::Degrees);
        assert_eq!(back.theme, ThemeMode::Light);
        assert_eq!(back.accent_idx, Some(2));
        assert_eq!(back.history_capacity, 123);
        assert_eq!(back.startup_mode, CalcMode::Finance);
        assert!(!back.show_history);
        assert_eq!(back.number_format, NumberFormat::Scientific);
        assert!(!back.history_auto_trim);
        assert_eq!(back.favorites.len(), 1);
        let _ = std::fs::remove_file(&dir);
        // Live state honors the persisted startup + format + trim flag.
        let s = crate::state::AppState::with_settings(back);
        assert_eq!(s.mode, CalcMode::Finance);
        assert_eq!(s.number_format, NumberFormat::Scientific);
        assert!(!s.settings.history_auto_trim);
    }

    #[test]
    fn phase8_settings_messages_wire_all_categories() {
        use crate::ods_theme::ThemeMode;
        use crate::state::{AngleUnit, NumberFormat};
        let mut a = app();
        // Appearance.
        OcalcApp::update_inner(&mut a.state, Message::ThemeChanged(ThemeMode::Light));
        assert_eq!(a.state.settings.theme, ThemeMode::Light);
        OcalcApp::update_inner(&mut a.state, Message::AccentChanged(Some(1)));
        assert_eq!(a.state.settings.accent_idx, Some(1));
        OcalcApp::update_inner(&mut a.state, Message::FontScaleChanged(1.25));
        assert_eq!(a.state.settings.font_scale, 1.25);
        // Calculation.
        OcalcApp::update_inner(&mut a.state, Message::AngleUnitChanged(AngleUnit::Degrees));
        assert_eq!(a.state.angle, AngleUnit::Degrees);
        OcalcApp::update_inner(&mut a.state, Message::PrecisionChanged(3));
        assert_eq!(a.state.precision, 3);
        OcalcApp::update_inner(
            &mut a.state,
            Message::NumberFormatChanged(NumberFormat::Scientific),
        );
        assert_eq!(a.state.number_format, NumberFormat::Scientific);
        // History.
        OcalcApp::update_inner(&mut a.state, Message::HistoryCapacityChanged(111));
        assert_eq!(a.state.settings.history_capacity, 111);
        OcalcApp::update_inner(&mut a.state, Message::HistoryAutoTrimChanged(false));
        assert!(!a.state.settings.history_auto_trim);
        // Layout.
        OcalcApp::update_inner(&mut a.state, Message::StartupModeChanged(CalcMode::Graph));
        assert_eq!(a.state.settings.startup_mode, CalcMode::Graph);
    }

    #[test]
    fn phase8_history_empty_search_and_favorites_flow() {
        let mut a = app();
        // Empty states: no history, no favorites, no matches.
        assert!(a.state.filtered_history().is_empty());
        assert!(a.state.favorites.is_empty());
        OcalcApp::update_inner(
            &mut a.state,
            Message::HistorySearchChanged("zzz-no-match".into()),
        );
        assert!(a.state.filtered_history().is_empty());
        OcalcApp::update_inner(&mut a.state, Message::HistorySearchChanged(String::new()));
        // Evaluate, star, reuse, unstar, clear.
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("1+2".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        assert_eq!(a.state.history.len(), 1);
        OcalcApp::update_inner(&mut a.state, Message::FavoriteToggleCurrent);
        assert_eq!(a.state.favorites.len(), 1);
        let fav_id = a.state.favorites[0].id;
        OcalcApp::update_inner(&mut a.state, Message::Clear);
        OcalcApp::update_inner(&mut a.state, Message::FavoriteReuse(fav_id));
        assert_eq!(a.state.input, "1+2");
        OcalcApp::update_inner(&mut a.state, Message::FavoriteToggleCurrent);
        assert!(a.state.favorites.is_empty());
        // History star path + clear-all favorites path.
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("3*3".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        let hid = a.state.history.all().last().unwrap().id;
        OcalcApp::update_inner(&mut a.state, Message::FavoriteToggleEntry(hid));
        assert_eq!(a.state.favorites.len(), 1);
        OcalcApp::update_inner(&mut a.state, Message::FavoritesClear);
        assert!(a.state.favorites.is_empty());
        // Search filters to the matching entry.
        OcalcApp::update_inner(&mut a.state, Message::HistorySearchChanged("3*3".into()));
        assert_eq!(a.state.filtered_history().len(), 1);
    }

    #[test]
    fn phase8_graph_hover_visibility_and_view_controls() {
        let mut a = app();
        // Hover readout set by the canvas, cleared on data refresh.
        OcalcApp::update_inner(&mut a.state, Message::GraphHover(1.0, 2.0));
        assert_eq!(a.state.graph.hover, Some((1.0, 2.0)));
        OcalcApp::update_inner(&mut a.state, Message::GraphZoomIn);
        assert!(a.state.graph.hover.is_none());
        assert!(a.state.graph.zoom > 1.0);
        OcalcApp::update_inner(&mut a.state, Message::GraphHover(0.5, 0.5));
        OcalcApp::update_inner(&mut a.state, Message::GraphToggleGrid);
        // Grid toggle never drops the hover (display-only).
        assert_eq!(a.state.graph.hover, Some((0.5, 0.5)));
        // Visibility: hiding keeps the slot, clears its samples.
        let id = a.state.graph.funcs[0].id;
        OcalcApp::update_inner(&mut a.state, Message::GraphToggleFunc(id));
        assert!(a
            .state
            .graph
            .funcs
            .iter()
            .find(|f| f.id == id)
            .unwrap()
            .samples
            .is_empty());
        OcalcApp::update_inner(&mut a.state, Message::GraphToggleFunc(id));
        assert!(!a
            .state
            .graph
            .funcs
            .iter()
            .find(|f| f.id == id)
            .unwrap()
            .samples
            .is_empty());
        // Pan/reset keep a valid ordered range.
        OcalcApp::update_inner(&mut a.state, Message::GraphPanLeft);
        OcalcApp::update_inner(&mut a.state, Message::GraphPanRight);
        OcalcApp::update_inner(&mut a.state, Message::GraphResetView);
        assert_eq!((a.state.graph.zoom, a.state.graph.pan), (1.0, 0.0));
        let (from, to) = a.state.graph.effective_range();
        assert!(from < to);
    }

    #[test]
    fn phase8_copy_and_clear_actions_notify() {
        let mut a = app();
        // Copy with nothing → error toast, no crash.
        OcalcApp::update_inner(&mut a.state, Message::CopyResult);
        assert!(a.state.toast.clone().is_some_and(|t| t.is_error));
        // Evaluate then copy → info toast.
        OcalcApp::update_inner(&mut a.state, Message::AppendInput("2+2".into()));
        OcalcApp::update_inner(&mut a.state, Message::Evaluate);
        OcalcApp::update_inner(&mut a.state, Message::CopyResult);
        assert!(a.state.toast.clone().is_some_and(|t| !t.is_error));
        // Clear history notifies; dismiss clears the toast.
        OcalcApp::update_inner(&mut a.state, Message::HistoryClear);
        assert!(a.state.history.is_empty());
        OcalcApp::update_inner(&mut a.state, Message::DismissToast);
        assert!(a.state.toast.is_none());
    }
}
