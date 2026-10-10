//! CAS view: symbolic expression input + rewrite buttons + collapsible
//! solver panel + result display + collapsible expression history.
//!
//! Every rewrite runs in `ocalc-cas` through `commands` (`cas_simplify`,
//! `cas_differentiate`, …); this view only edits text and selections.

use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::AppState;
use crate::views::{accent_key, empty_state, heading, hint, result_panel};

/// Collapsible section header with a Show/Hide toggle.
fn section_header(
    title: &str,
    expanded: bool,
    toggle: Message,
    ty: crate::ods_theme::Typography,
) -> Element<'_, Message> {
    let label = if expanded { "Hide" } else { "Show" };
    row![
        heading(title, ty.body),
        iced::widget::Space::new().width(Length::Fill),
        button(text(label).size(ty.small))
            .on_press(toggle)
            .style(subtle_button()),
    ]
    .align_y(Alignment::Center)
    .spacing(6)
    .into()
}

/// CAS mode: expression editor, rewrite buttons, solvers, history.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let danger = state.theme.colors.danger;
    let muted = state.theme.colors.muted;
    let c = &state.cas;

    let input_box = text_input("Expression, e.g. (x + 1)^2", &c.input)
        .on_input(Message::CasInputChanged)
        .on_submit(Message::CasSimplify)
        .id("cas-input")
        .size(ty.body)
        .padding(sp.md);

    let var_row = row![
        text("Variable:").size(ty.small),
        text_input("x", &c.var)
            .on_input(Message::CasVarChanged)
            .on_submit(Message::CasDifferentiate)
            .size(ty.small),
    ]
    .align_y(Alignment::Center)
    .spacing(6);

    let ops = row![
        accent_key("Simplify", Message::CasSimplify, state.theme.colors.accent),
        button(text("Expand").size(ty.small))
            .on_press(Message::CasExpand)
            .style(subtle_button()),
        button(text("Factor").size(ty.small))
            .on_press(Message::CasFactor)
            .style(subtle_button()),
        button(text("d/dx").size(ty.small))
            .on_press(Message::CasDifferentiate)
            .style(subtle_button()),
        button(text("∫dx").size(ty.small))
            .on_press(Message::CasIntegrate)
            .style(subtle_button()),
    ]
    .spacing(sp.xs);

    let result = result_panel(c.result.clone(), &c.error, "—", ty, danger, muted);

    // Solvers: linear a·x + b = 0 and quadratic a·x² + b·x + c = 0.
    let mut solvers = column![section_header(
        "Solve",
        c.show_solvers,
        Message::CasToggleSolvers,
        ty
    )]
    .spacing(sp.xs);
    if c.show_solvers {
        solvers = solvers.push(
            row![
                text("a·x + b = 0:").size(ty.small),
                text_input("a", &c.lin_a)
                    .on_input(move |s| Message::CasLinChanged(0, s))
                    .size(ty.small),
                text_input("b", &c.lin_b)
                    .on_input(move |s| Message::CasLinChanged(1, s))
                    .size(ty.small),
                button(text("Solve").size(ty.small))
                    .on_press(Message::CasSolveLinear)
                    .style(subtle_button()),
            ]
            .align_y(Alignment::Center)
            .spacing(6),
        );
        solvers = solvers.push(
            row![
                text("a·x² + b·x + c = 0:").size(ty.small),
                text_input("a", &c.quad_a)
                    .on_input(move |s| Message::CasQuadChanged(0, s))
                    .size(ty.small),
                text_input("b", &c.quad_b)
                    .on_input(move |s| Message::CasQuadChanged(1, s))
                    .size(ty.small),
                text_input("c", &c.quad_c)
                    .on_input(move |s| Message::CasQuadChanged(2, s))
                    .size(ty.small),
                button(text("Solve").size(ty.small))
                    .on_press(Message::CasSolveQuadratic)
                    .style(subtle_button()),
            ]
            .align_y(Alignment::Center)
            .spacing(6),
        );
    }
    let solvers: Element<'_, Message> = solvers.into();

    // Expression history (newest last, capped by state).
    let mut history = column![section_header(
        "History",
        c.show_history,
        Message::CasToggleHistory,
        ty
    ),]
    .spacing(2);
    if c.show_history {
        history = history.push(
            row![
                iced::widget::Space::new().width(Length::Fill),
                button(text("Clear").size(ty.small))
                    .on_press(Message::CasHistoryClear)
                    .style(subtle_button()),
            ]
            .align_y(Alignment::Center)
            .spacing(6),
        );
        if c.history.is_empty() {
            history = history.push(empty_state(
                "No calculations yet — simplify an expression above.",
                ty.small,
                muted,
            ));
        }
        for entry in &c.history {
            history = history.push(
                row![
                    text(format!("{}: {} = {}", entry.op, entry.input, entry.output))
                        .size(ty.small)
                        .width(Length::Fill),
                    button(text("Use").size(ty.small))
                        .on_press(Message::CasReuse(entry.id))
                        .style(subtle_button()),
                ]
                .align_y(Alignment::Center)
                .spacing(6),
            );
        }
    }
    let history = scrollable(history).height(Length::Fill);

    column![
        text("CAS — computer algebra").size(ty.heading),
        input_box,
        var_row,
        ops,
        result,
        solvers,
        history,
        hint(
            "Variables x y z, operators + - * / ^, parens, sin(…) cos(…) log(…).",
            ty.small
        ),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
