//! Physics view: formula selector + input cards + unit display + result
//! panel + collapsible SI constant reference.
//!
//! Formulas compute in `ocalc-physics` through `commands::physics_compute`;
//! this view only edits the selection and input text.

use crate::commands;
use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::{AppState, PhysicsFormula};
use crate::views::{hint, labeled_field, result_panel, section_card};
use iced::widget::{button, column, pick_list, row, text};
use iced::{Alignment, Element, Length};
use ocalc_engine::Value;

/// Physics mode: formula picker, labeled inputs, result with unit.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let danger = state.theme.colors.danger;
    let muted = state.theme.colors.muted;
    let p = &state.physics;

    let picker = pick_list(
        PhysicsFormula::ALL.to_vec(),
        Some(p.formula),
        Message::PhysicsFormulaChanged,
    )
    .placeholder("formula");

    let mut inputs = column![labeled_field(
        p.formula.a_label(),
        &p.a_text,
        Some("physics-a"),
        Message::PhysicsAChanged,
        ty.small,
    )]
    .spacing(sp.xs);
    if let Some(b_label) = p.formula.b_label() {
        inputs = inputs.push(labeled_field(
            b_label,
            &p.b_text,
            Some("physics-b"),
            Message::PhysicsBChanged,
            ty.small,
        ));
    }

    let result = result_panel(
        p.result.clone().map(|v| format!("{v} {}", p.unit)),
        &p.error,
        "—",
        ty,
        danger,
        muted,
    );

    // Read-only SI constant reference from the physics crate.
    let toggle = if p.show_constants { "Hide" } else { "Show" };
    let mut consts = column![row![
        crate::views::heading("Constants (SI)", ty.body),
        iced::widget::Space::new().width(Length::Fill),
        button(text(toggle).size(ty.small))
            .on_press(Message::PhysicsToggleConstants)
            .style(subtle_button()),
    ]
    .align_y(Alignment::Center)
    .spacing(6),]
    .spacing(2);
    if p.show_constants {
        for c in commands::physics_constants() {
            let value = state.fmt_value(&Value::Float(c.value));
            consts = consts.push(
                row![
                    text(c.name).size(ty.small).width(Length::Fill),
                    text(format!("{value} {}", c.unit)).size(ty.small),
                ]
                .spacing(8),
            );
        }
    }
    let consts: Element<'_, Message> = consts.into();

    column![
        text("Physics").size(ty.heading),
        picker,
        section_card("Inputs", inputs.into(), ty, sp),
        result,
        consts,
        hint("All formulas use SI units (kg, m, s, V, A, Ω).", ty.small),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
