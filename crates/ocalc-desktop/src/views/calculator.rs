//! Standard calculator view: display + memory row + keypad.
//!
//! Memory (MC/MR/M+/M-) lives in [`AppState::memory`](crate::state::AppState)
//! and combines through public [`Value`] arithmetic in `commands.rs`.

use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::AppState;
use crate::views::{accent_key, display_panel, key};

/// Standard mode: memory row, display, keypad.
///
/// Phase 8: keyboard-first workflow is surfaced with a hint line; the
/// expression editor keeps native cursor navigation (arrow keys, Home/End
/// via the focused `text_input`), and result actions (copy/star) stay one
/// click away.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let display = display_panel(
        &state.input,
        &state.display,
        &state.error,
        state.theme.colors.danger,
        ty,
        sp,
    );

    // Memory row: MC MR MS M+ M- (+ M indicator handled in the header).
    let mem = row![
        iced::widget::button(iced::widget::text("MC").size(ty.small))
            .on_press(Message::MemoryClear)
            .style(subtle_button())
            .width(Length::Fill),
        iced::widget::button(iced::widget::text("MR").size(ty.small))
            .on_press(Message::MemoryRecall)
            .style(subtle_button())
            .width(Length::Fill),
        iced::widget::button(iced::widget::text("MS").size(ty.small))
            .on_press(Message::MemoryStore)
            .style(subtle_button())
            .width(Length::Fill),
        iced::widget::button(iced::widget::text("M+").size(ty.small))
            .on_press(Message::MemoryAdd)
            .style(subtle_button())
            .width(Length::Fill),
        iced::widget::button(iced::widget::text("M−").size(ty.small))
            .on_press(Message::MemorySub)
            .style(subtle_button())
            .width(Length::Fill),
    ]
    .spacing(sp.xs);

    // Result actions: copy the display to the clipboard, star the current
    // calculation for the favorites shelf in the history sidebar.
    let star_label = if state.is_favorited(state.input.trim()) {
        "★ Starred"
    } else {
        "☆ Star"
    };
    let actions = row![
        button(text("⧉ Copy result").size(ty.small))
            .on_press(Message::CopyResult)
            .style(subtle_button())
            .width(Length::Fill),
        button(text(star_label).size(ty.small))
            .on_press(Message::FavoriteToggleCurrent)
            .style(subtle_button())
            .width(Length::Fill),
    ]
    .spacing(sp.xs)
    .align_y(Alignment::Center);

    // 4×5 keypad grid. Literals appended verbatim; `=` evaluates.
    let pad = column![
        row![
            key("7", Message::AppendInput("7".into())),
            key("8", Message::AppendInput("8".into())),
            key("9", Message::AppendInput("9".into())),
            key("/", Message::AppendInput("/".into())),
            key("C", Message::Clear),
        ]
        .spacing(sp.xs),
        row![
            key("4", Message::AppendInput("4".into())),
            key("5", Message::AppendInput("5".into())),
            key("6", Message::AppendInput("6".into())),
            key("*", Message::AppendInput("*".into())),
            key("CE", Message::ClearEntry),
        ]
        .spacing(sp.xs),
        row![
            key("1", Message::AppendInput("1".into())),
            key("2", Message::AppendInput("2".into())),
            key("3", Message::AppendInput("3".into())),
            key("-", Message::AppendInput("-".into())),
            key("⌫", Message::Backspace),
        ]
        .spacing(sp.xs),
        row![
            key("0", Message::AppendInput("0".into())),
            key(".", Message::AppendInput(".".into())),
            accent_key("=", Message::Evaluate, state.theme.colors.accent),
            key("+", Message::AppendInput("+".into())),
            key("^", Message::AppendInput("^".into())),
        ]
        .spacing(sp.xs),
        row![
            key("(", Message::AppendInput("(".into())),
            key(")", Message::AppendInput(")".into())),
            key("%", Message::AppendInput("%".into())),
            key("!", Message::AppendInput("!".into())),
            key("±", Message::Negate),
        ]
        .spacing(sp.xs),
    ]
    .spacing(sp.xs);

    let keyboard_hint: Element<'_, Message> =
        text("Enter evaluates • Esc clears • ←/→ move cursor • Ctrl+1…9 switch modes")
            .size(ty.tiny)
            .color(state.theme.colors.muted)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .into();

    column![mem, display, actions, pad, keyboard_hint]
        .spacing(sp.sm)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
