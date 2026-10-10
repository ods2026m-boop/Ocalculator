//! Mode views. Each `view(state)` builds an [`Element`] from [`AppState`]
//! without computing anything itself — math lives in `commands.rs`.
//!
//! Shared primitives ([`key`], [`accent_key`], [`display_panel`],
//! [`heading`], [`hint`]) live here; the shell, history sidebar and settings
//! panel have their own modules.

pub mod calculator;
pub mod cas;
pub mod chemistry;
pub mod converter;
pub mod finance;
pub mod graph;
pub mod history;
pub mod physics;
pub mod programmer;
pub mod scientific;
pub mod settings;
pub mod shell;

use iced::widget::{button, column, container, text, text_input};
use iced::{Color, Element, Length};

use crate::messages::Message;
use crate::ods_theme::{primary_button, subtle_button, Spacing, Typography};

/// Keypad button helper (quiet style with hover/press feedback).
pub fn key(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(
        text(label)
            .size(18)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(msg)
    .style(subtle_button())
    .width(Length::Fill)
    .into()
}

/// Accent button helper (evaluate, apply): ODS accent + white text.
pub fn accent_key(
    label: &'static str,
    msg: Message,
    accent: iced::Color,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(18)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(msg)
    .style(primary_button(accent))
    .width(Length::Fill)
    .into()
}

/// Expression display + result/error readout (shared).
///
/// `type_` and `sp` are taken by value ([`Typography`] / [`Spacing`] are
/// `Copy`) so the panel never borrows view-locals.
///
/// Phase 8 polish: consistent card padding, muted empty placeholder with
/// keyboard hint, danger-styled first-line error with a correction hint.
pub fn display_panel<'a>(
    input: &'a str,
    display: &'a str,
    error: &'a Option<String>,
    danger: Color,
    type_: Typography,
    sp: Spacing,
) -> Element<'a, Message> {
    let input_box = text_input("Enter expression… e.g. sin(30) + sqrt(25) * 2", input)
        .on_input(Message::InputChanged)
        .on_submit(Message::Evaluate)
        .id("calc-input")
        .size(type_.body + 2.0)
        .padding(sp.md);

    let result: Element<'_, Message> = if !display.is_empty() {
        text(format!("= {display}")).size(type_.display).into()
    } else if error.is_none() {
        text("Type an expression, then press Enter")
            .size(type_.body)
            .into()
    } else {
        text("—").size(type_.display).into()
    };

    let mut col = column![input_box, result].spacing(sp.xs);
    if let Some(err) = error {
        // First line only in the panel; full caret text is in the toast.
        let first = err.lines().next().unwrap_or("error");
        col = col.push(error_text(format!("⚠ {first}"), type_.small, danger));
        col = col
            .push(text("Check parentheses, function names and argument counts.").size(type_.tiny));
    }
    container(col).width(Length::Fill).padding(sp.xs).into()
}

/// Small section heading (owned: no borrow retained, temporaries allowed).
pub fn heading(label: impl Into<String>, size: f32) -> Element<'static, Message> {
    text(label.into()).size(size).into()
}

/// Muted hint line (owned: no borrow retained, temporaries allowed).
pub fn hint(label: impl Into<String>, size: f32) -> Element<'static, Message> {
    text(label.into()).size(size).into()
}

/// Grouped card: titled section with consistent padding for input groups
/// (physics formulas, finance calculators, CAS solvers).
pub fn section_card<'a>(
    title: &'a str,
    content: Element<'a, Message>,
    type_: Typography,
    sp: Spacing,
) -> Element<'a, Message> {
    container(column![heading(title, type_.body), content].spacing(sp.xs))
        .padding(sp.md)
        .width(Length::Fill)
        .into()
}

/// Consistent result readout: value at display size, danger-colored error,
/// or a muted empty placeholder.
pub fn result_panel<'a>(
    value: Option<String>,
    error: &'a Option<String>,
    empty: &'a str,
    type_: Typography,
    danger: Color,
    muted: Color,
) -> Element<'a, Message> {
    match (value, error) {
        (Some(v), _) => text(format!("= {v}")).size(type_.display * 0.875).into(),
        (None, Some(err)) => {
            let first = err.lines().next().unwrap_or("error");
            column![
                error_text(format!("⚠ {first}"), type_.small, danger),
                text("Adjust the inputs and try again.")
                    .size(type_.tiny)
                    .color(muted),
            ]
            .spacing(2)
            .into()
        }
        (None, None) => empty_state(empty.to_owned(), type_.body, muted),
    }
}

/// Consistent error line in the danger color.
pub fn error_text(msg: impl Into<String>, size: f32, danger: Color) -> Element<'static, Message> {
    text(msg.into())
        .size(size)
        .color(danger)
        .width(Length::Fill)
        .into()
}

/// Consistent empty-state placeholder in the muted color.
///
/// Centered and full-width so lists, histories and browsers all share the
/// same calm "nothing here yet" presentation.
pub fn empty_state(label: impl Into<String>, size: f32, muted: Color) -> Element<'static, Message> {
    text(label.into())
        .size(size)
        .color(muted)
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

/// Labeled numeric field: fixed label plus fill input (shared by the
/// physics and finance forms). `id` optionally tags the input for
/// keyboard-focus navigation (`None` for secondary fields).
pub fn labeled_field<'a>(
    label: &'a str,
    value: &'a str,
    id: Option<&'static str>,
    on_input: impl Fn(String) -> Message + 'a,
    small: f32,
) -> Element<'a, Message> {
    use iced::widget::row;
    use iced::Alignment;
    let mut input = text_input("0", value)
        .on_input(on_input)
        .size(small)
        .padding(6);
    if let Some(id) = id {
        input = input.id(id);
    }
    row![text(label).size(small).width(Length::Fixed(140.0)), input,]
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
}
