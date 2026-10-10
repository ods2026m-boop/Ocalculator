//! Application shell: header, mode toolbar, content, sidebars, toast,
//! status bar. Responsive: the calculator area fills; sidebars collapse via
//! toolbar toggles so narrow/resized windows stay usable.

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::{AppState, CalcMode};
use crate::views::{self, history, settings};

/// Full window layout.
///
/// Phase 8: consistent ODS spacing (`sm` gaps, `md` outer padding), the
/// content row fills remaining space so resized windows reflow instead of
/// clipping, and the status bar stays pinned at the bottom.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let sp = state.theme.spacing;
    let header = header_row(state);
    let toolbar = toolbar_row(state);
    let content = content_row(state);
    let toast = toast_row(state);
    let status = status_bar(state);

    let mut col = column![header, toolbar, content]
        .spacing(sp.sm)
        .padding(sp.md);
    if let Some(notification) = toast {
        col = col.push(notification);
    }
    col = col.push(status);
    col.height(Length::Fill).into()
}

/// Header: title, memory indicator, sidebar toggles.
fn header_row(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let title = text("Ocalculator").size(ty.heading);
    let subtitle = text(state.mode.label())
        .size(ty.small)
        .color(state.theme.colors.muted);
    let mem: Element<'_, Message> = if state.memory.is_some() {
        text("● M")
            .size(ty.small)
            .color(state.theme.colors.success)
            .into()
    } else {
        text("○ M")
            .size(ty.small)
            .color(state.theme.colors.muted)
            .into()
    };
    let toggles = row![
        button(
            text(if state.settings.show_history {
                "Hide history"
            } else {
                "History"
            })
            .size(ty.small)
        )
        .on_press(Message::ToggleHistory)
        .style(subtle_button()),
        button(
            text(if state.settings.show_settings {
                "Hide settings"
            } else {
                "Settings"
            })
            .size(ty.small)
        )
        .on_press(Message::ToggleSettings)
        .style(subtle_button()),
    ]
    .spacing(sp.xs);
    row![
        title,
        subtitle,
        mem,
        iced::widget::Space::new().width(Length::Fill),
        toggles,
    ]
    .align_y(Alignment::Center)
    .spacing(sp.sm)
    .into()
}

/// Toolbar: mode selector with shortcut hints + quick actions.
///
/// Main modes stay in the first row; the Phase 7 scientific category
/// (CAS / Physics / Chemistry / Finance) gets its own labeled row so the
/// existing layout is preserved.
///
/// Phase 8: the active mode uses the accent [`primary_button`] style while
/// inactive modes stay quiet ([`subtle_button`]), giving instant navigation
/// feedback without changing the tab order or shortcuts.
fn toolbar_row(state: &AppState) -> Element<'_, Message> {
    use crate::ods_theme::primary_button;
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let accent = state.theme.colors.accent;
    let mut modes = row![].spacing(sp.xs);
    for mode in CalcMode::MAIN {
        let active = mode == state.mode;
        let label = format!("{} ({})", mode.label(), mode.shortcut_digit());
        let btn = button(text(label).size(ty.small))
            .on_press(Message::ModeChanged(mode))
            .width(Length::Fill);
        modes = modes.push(if active {
            btn.style(primary_button(accent))
        } else {
            btn.style(subtle_button())
        });
    }
    let mut science = row![text("Scientific:")
        .size(ty.small)
        .color(state.theme.colors.muted)]
    .align_y(Alignment::Center)
    .spacing(sp.xs);
    for mode in CalcMode::SCIENTIFIC {
        let active = mode == state.mode;
        let label = format!("{} ({})", mode.label(), mode.shortcut_digit());
        let btn = button(text(label).size(ty.small))
            .on_press(Message::ModeChanged(mode))
            .width(Length::Fill);
        science = science.push(if active {
            btn.style(primary_button(accent))
        } else {
            btn.style(subtle_button())
        });
    }
    let quick = row![
        button(text("CE").size(ty.small))
            .on_press(Message::ClearEntry)
            .style(subtle_button()),
        button(text("C").size(ty.small))
            .on_press(Message::Clear)
            .style(subtle_button()),
    ]
    .spacing(sp.xs);
    column![
        row![container(modes).width(Length::Fill), quick].spacing(sp.sm),
        container(science).width(Length::Fill),
    ]
    .spacing(sp.xs)
    .into()
}

/// Content: calculator area (fills) + optional sidebars (fixed).
///
/// The row fills remaining window height so narrow/resized windows keep the
/// status bar visible and the calculator area reflows; sidebars stay fixed
/// width and collapse via header toggles.
fn content_row(state: &AppState) -> Element<'_, Message> {
    let sp = state.theme.spacing;
    let content: Element<'_, Message> = match state.mode {
        CalcMode::Standard => views::calculator::view(state),
        CalcMode::Scientific => views::scientific::view(state),
        CalcMode::Programmer => views::programmer::view(state),
        CalcMode::Converter => views::converter::view(state),
        CalcMode::Graph => views::graph::view(state),
        CalcMode::Cas => views::cas::view(state),
        CalcMode::Physics => views::physics::view(state),
        CalcMode::Chemistry => views::chemistry::view(state),
        CalcMode::Finance => views::finance::view(state),
    };
    let mut content_row =
        row![container(content).width(Length::Fill).height(Length::Fill)].spacing(sp.md);
    if state.settings.show_history {
        content_row = content_row.push(history::view(state));
    }
    if state.settings.show_settings {
        content_row = content_row.push(settings::view(state));
    }
    container(content_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Boxed button style (the ODS styles are distinct opaque types).
type BoxedButtonStyle =
    Box<dyn Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style>;

/// Transient notification with dismiss button.
///
/// Errors get a ⚠ prefix and the danger style; info gets ℹ and the accent
/// style. The message always fills so long errors wrap instead of clipping.
fn toast_row(state: &AppState) -> Option<Element<'_, Message>> {
    state.toast.as_ref().map(|t| {
        let ty = state.theme.scaled_typography();
        let sp = state.theme.spacing;
        let style: BoxedButtonStyle = if t.is_error {
            Box::new(crate::ods_theme::danger_button(state.theme.colors.danger))
        } else {
            Box::new(crate::ods_theme::primary_button(state.theme.colors.accent))
        };
        let icon = if t.is_error { "⚠" } else { "ℹ" };
        row![
            container(text(format!("{icon} {}", t.message)).size(ty.small)).width(Length::Fill),
            button(text("Dismiss").size(ty.small))
                .on_press(Message::DismissToast)
                .style(style),
        ]
        .align_y(Alignment::Center)
        .spacing(sp.sm)
        .into()
    })
}

/// Status bar: mode • angle • precision • format • history • favorites •
/// eval stats. Muted and tiny so it reads as chrome, not content.
fn status_bar(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let angle = state.angle.label();
    let timing = match state.status.last_duration_us {
        Some(us) => format!("{us} µs"),
        None => "—".to_owned(),
    };
    let label = format!(
        "{}  •  {}  •  precision {}  •  {}  •  history {}  •  ★ {}  •  evals {} ({})",
        state.mode.label(),
        angle,
        state.precision,
        state.number_format.label().to_lowercase(),
        state.history.len(),
        state.favorites.len(),
        state.status.eval_count,
        timing,
    );
    container(text(label).size(ty.tiny).color(state.theme.colors.muted))
        .width(Length::Fill)
        .padding(sp.xs)
        .into()
}
