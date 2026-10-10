//! Settings panel: categorized appearance / calculation / history /
//! modules / shortcuts.
//!
//! Phase 8 complete settings page. Every control maps to a persisted
//! [`Settings`](crate::state::Settings) field and autosaves via
//! `app.rs` (`affects_settings`); the explicit Save button is a manual
//! checkpoint with toast feedback. Grouped with headings and scrollable so
//! narrow windows stay usable.

use iced::widget::{
    button, checkbox, column, pick_list, row, scrollable, slider, text, text_input,
};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::{primary_button, subtle_button, ACCENT_PRESETS};
use crate::state::{AngleUnit, AppState, CalcMode, NumberFormat};

/// Shortcut reference rows: (keys, action).
///
/// Covers every global shortcut: evaluation, editing, all nine
/// `Ctrl+1…9` mode switches, focus movement and history reuse.
pub const SHORTCUTS: [(&str, &str); 17] = [
    ("Enter / =", "Evaluate expression"),
    ("Esc", "Clear input + result"),
    ("Backspace", "Delete last character"),
    ("Delete", "Clear entry (keep result)"),
    ("0–9 + - * / % ^ ( ) . !", "Append to input"),
    ("Ctrl+1", "Standard mode"),
    ("Ctrl+2", "Scientific mode"),
    ("Ctrl+3", "Programmer mode"),
    ("Ctrl+4", "Converter mode"),
    ("Ctrl+5", "Graph mode"),
    ("Ctrl+6", "CAS mode"),
    ("Ctrl+7", "Physics mode"),
    ("Ctrl+8", "Chemistry mode"),
    ("Ctrl+9", "Finance mode"),
    ("Tab / Shift+Tab", "Move keyboard focus"),
    ("Click history entry", "Reuse expression"),
    ("☆ / ★", "Star / unstar favorite"),
];

/// Settings panel (fixed width, scrollable, collapsible via header toggle).
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let muted = state.theme.colors.muted;
    let s = &state.settings;

    // -- Appearance ------------------------------------------------------
    let mut theme_row = row![text("Theme:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(sp.xs);
    for mode in crate::ods_theme::ThemeMode::ALL {
        let label = if s.theme == mode {
            format!("[{}]", mode.label())
        } else {
            mode.label().to_owned()
        };
        theme_row = theme_row.push(
            button(text(label).size(ty.small))
                .on_press(Message::ThemeChanged(mode))
                .style(subtle_button()),
        );
    }

    let mut accent_row = row![text("Accent:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(sp.xs);
    accent_row = accent_row.push(
        button(
            text(if s.accent_idx.is_none() {
                "[Auto]"
            } else {
                "Auto"
            })
            .size(ty.small),
        )
        .on_press(Message::AccentChanged(None))
        .style(subtle_button()),
    );
    for (i, _preset) in ACCENT_PRESETS.iter().enumerate() {
        let selected = s.accent_idx == Some(i);
        let label = if selected {
            format!("[{}]", i + 1)
        } else {
            format!("{}", i + 1)
        };
        accent_row = accent_row.push(
            button(text(label).size(ty.small))
                .on_press(Message::AccentChanged(Some(i)))
                .style(subtle_button()),
        );
    }

    let font_row = column![
        text(format!("Font size: {:.0}%", s.font_scale * 100.0)).size(ty.small),
        slider(0.875..=1.5, s.font_scale, Message::FontScaleChanged).step(0.125_f32),
    ]
    .spacing(sp.xs);

    // -- Calculation -----------------------------------------------------
    let mut angle_row = row![text("Angle:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(sp.xs);
    for unit in AngleUnit::ALL {
        let label = if state.angle == unit {
            format!("[{}]", unit.label())
        } else {
            unit.label().to_owned()
        };
        angle_row = angle_row.push(
            button(text(label).size(ty.small))
                .on_press(Message::AngleUnitChanged(unit))
                .style(subtle_button()),
        );
    }
    let angle_hint = if state.angle == AngleUnit::Gradians {
        "100 grad = 90° — rewritten to radians at the UI boundary."
    } else {
        "Degrees and radians run in the engine directly."
    };

    let prec_row = row![
        text(format!("Precision: {} dp", state.precision)).size(ty.small),
        button(text("−").size(ty.small))
            .on_press(Message::PrecisionChanged(state.precision.saturating_sub(1)))
            .style(subtle_button()),
        button(text("+").size(ty.small))
            .on_press(Message::PrecisionChanged(
                (state.precision + 1).min(crate::commands::MAX_PRECISION)
            ))
            .style(subtle_button()),
    ]
    .align_y(Alignment::Center)
    .spacing(sp.xs);

    let mut format_row = row![text("Format:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(sp.xs);
    for fmt in [NumberFormat::Standard, NumberFormat::Scientific] {
        let label = if state.number_format == fmt {
            format!("[{}]", fmt.label())
        } else {
            fmt.label().to_owned()
        };
        format_row = format_row.push(
            button(text(label).size(ty.small))
                .on_press(Message::NumberFormatChanged(fmt))
                .style(subtle_button()),
        );
    }

    // -- History ---------------------------------------------------------
    let cap_row = column![
        text(format!("Maximum entries: {}", s.history_capacity)).size(ty.small),
        slider(10.0..=2000.0, s.history_capacity as f32, move |v| {
            Message::HistoryCapacityChanged(v as usize)
        })
        .step(10.0_f32),
    ]
    .spacing(sp.xs);
    let auto_row = column![
        checkbox(s.history_auto_trim)
            .label("Auto cleanup: trim oldest beyond the maximum")
            .on_toggle(Message::HistoryAutoTrimChanged),
        text(if s.history_auto_trim {
            "New results evict the oldest entries automatically."
        } else {
            "History grows unbounded until you press Clear."
        })
        .size(ty.tiny),
    ]
    .spacing(sp.xs);

    // -- Startup & layout ------------------------------------------------
    let startup = row![
        text("Startup mode:").size(ty.small),
        pick_list(
            CalcMode::ALL.to_vec(),
            Some(s.startup_mode),
            Message::StartupModeChanged,
        )
        .placeholder("mode"),
    ]
    .align_y(Alignment::Center)
    .spacing(sp.xs);

    let sidebars = column![
        checkbox(s.show_history)
            .label("History sidebar")
            .on_toggle(|_| Message::ToggleHistory),
        checkbox(s.show_settings)
            .label("Settings panel")
            .on_toggle(|_| Message::ToggleSettings),
    ]
    .spacing(sp.xs);

    let save = button(text("Save settings").size(ty.small))
        .on_press(Message::SettingsSave)
        .style(primary_button(state.theme.colors.accent))
        .width(Length::Fill);

    // -- Modules ---------------------------------------------------------
    let mut mods = column![text("Scientific modules").size(ty.body)].spacing(sp.xs);
    for (id, desc, funcs, enabled) in state.scientific_modules() {
        let toggle_label = if enabled { "On" } else { "Off" };
        mods = mods.push(
            row![
                text(format!("{id} ({} fn)", funcs.len()))
                    .size(ty.small)
                    .width(Length::Fill),
                button(text(toggle_label).size(ty.small))
                    .on_press(Message::ModuleToggled(id.clone()))
                    .style(subtle_button()),
            ]
            .align_y(Alignment::Center)
            .spacing(sp.xs),
        );
        mods = mods.push(text(desc).size(ty.tiny));
        let _ = funcs;
    }
    let hits = state.filtered_functions();
    let shown: Vec<String> = hits.into_iter().take(20).collect();
    let hits_text = if shown.is_empty() {
        "No matching functions.".to_owned()
    } else {
        shown.join(", ")
    };
    let func_search = column![
        text("Function search").size(ty.body),
        text_input("Search functions… e.g. force", &state.function_search)
            .on_input(Message::FunctionSearchChanged)
            .size(ty.small),
        text(hits_text).size(ty.tiny),
    ]
    .spacing(sp.xs);

    // -- Shortcuts -------------------------------------------------------
    let mut shortcuts = column![text("Keyboard shortcuts").size(ty.body)].spacing(sp.xs);
    for (keys, action) in SHORTCUTS {
        shortcuts = shortcuts.push(
            row![
                text(keys).size(ty.tiny).width(Length::Fill),
                text(action).size(ty.tiny).color(muted),
            ]
            .spacing(sp.sm),
        );
    }

    let body = column![
        text("Settings").size(ty.heading),
        text("Changes apply instantly and autosave to disk.").size(ty.small),
        text("Appearance").size(ty.body),
        theme_row,
        accent_row,
        font_row,
        text("Calculation").size(ty.body),
        angle_row,
        text(angle_hint).size(ty.tiny),
        prec_row,
        format_row,
        text("Precision rounds Float/Decimal results; integers stay exact.").size(ty.tiny),
        text("History").size(ty.body),
        cap_row,
        auto_row,
        text("Startup & layout").size(ty.body),
        startup,
        sidebars,
        save,
        mods,
        func_search,
        shortcuts,
    ]
    .spacing(sp.sm)
    .width(Length::Fill);

    scrollable(body)
        .height(Length::Fill)
        .width(Length::Fixed(340.0))
        .into()
}
