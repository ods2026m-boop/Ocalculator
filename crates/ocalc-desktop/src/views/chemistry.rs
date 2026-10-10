//! Chemistry view: formula input + molar mass + per-element breakdown +
//! element info panel + periodic-table browser.
//!
//! Analysis runs in `ocalc-chemistry` through `commands` (`chemistry_analyze`,
//! `chemistry_element`, `filter_elements`); this view only edits text and
//! the element selection.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use ocalc_engine::Value;

use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::AppState;
use crate::views::{empty_state, heading, hint, result_panel, section_card};

/// Elements per browser row.
const BROWSER_COLS: usize = 10;

/// Chemistry mode: formula analysis, element card, table browser.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let danger = state.theme.colors.danger;
    let muted = state.theme.colors.muted;
    let c = &state.chemistry;

    let formula_box = text_input("Formula, e.g. H2O or Ca(OH)2", &c.formula)
        .on_input(Message::ChemistryFormulaChanged)
        .on_submit(Message::Nop)
        .id("chem-formula")
        .size(ty.body)
        .padding(sp.md);

    let mass = result_panel(
        c.result_mass
            .map(|m| format!("{} g/mol", state.fmt_value(&Value::Float(m)))),
        &c.error,
        "Enter a formula to analyze it.",
        ty,
        danger,
        muted,
    );

    // Per-element breakdown as mini-cards: symbol button + share readout.
    let mut breakdown = column![heading("Composition", ty.body)].spacing(sp.xs);
    if c.breakdown.is_empty() {
        breakdown = breakdown.push(empty_state(
            "No elements yet — a valid formula lists each element here.",
            ty.small,
            muted,
        ));
    }
    for part in &c.breakdown {
        let mass_text = state.fmt_value(&Value::Float(part.mass));
        let pct_text = state.fmt_value(&Value::Float(part.percent));
        breakdown = breakdown.push(
            container(
                row![
                    button(text(part.symbol.clone()).size(ty.small))
                        .on_press(Message::ChemistrySelect(part.symbol.clone()))
                        .style(subtle_button()),
                    column![
                        text(format!("{} ×{} (# {})", part.name, part.count, part.number))
                            .size(ty.small),
                        text(format!("{mass_text} g/mol · {pct_text} %"))
                            .size(ty.tiny)
                            .color(muted),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                ]
                .align_y(Alignment::Center)
                .spacing(6),
            )
            .padding(sp.sm)
            .width(Length::Fill),
        );
    }

    // Element information panel for the current selection.
    let no_element = format!(
        "No element {:?} — pick one below.",
        state.chemistry.selected
    );
    let info: Element<'_, Message> = match state.chemistry_selected_card() {
        Some(card) => {
            let mass_text = state.fmt_value(&Value::Float(card.mass));
            section_card(
                "Element",
                text(format!(
                    "{} — {} (#{}), {mass_text} g/mol",
                    card.symbol, card.name, card.number
                ))
                .size(ty.small)
                .into(),
                ty,
                sp,
            )
        }
        None => section_card("Element", empty_state(no_element, ty.small, muted), ty, sp),
    };

    // Periodic-table browser: search + symbol grid.
    let search = text_input("Search table… e.g. oxy or 8", &c.table_search)
        .on_input(Message::ChemistrySearchChanged)
        .size(ty.small);
    let entries = state.chemistry_filtered_table();
    let no_match = format!("No elements match {:?}.", c.table_search.trim());
    let grid: Element<'_, Message> = if entries.is_empty() {
        empty_state(no_match, ty.small, muted)
    } else {
        let mut grid = column![].spacing(sp.xs);
        for chunk in entries.chunks(BROWSER_COLS) {
            let mut r = row![].spacing(sp.xs);
            for card in chunk {
                let label = if card.symbol == c.selected {
                    format!("[{}]", card.symbol)
                } else {
                    card.symbol.clone()
                };
                r = r.push(
                    button(text(label).size(ty.small))
                        .on_press(Message::ChemistrySelect(card.symbol.clone()))
                        .style(subtle_button())
                        .width(Length::Fill),
                );
            }
            grid = grid.push(r);
        }
        grid.into()
    };
    let browser = column![
        heading("Periodic table", ty.body),
        search,
        scrollable(grid).height(Length::Fixed(160.0)),
    ]
    .spacing(4);

    column![
        text("Chemistry").size(ty.heading),
        formula_box,
        mass,
        breakdown,
        info,
        browser,
        hint(
            "Groups (OH)2, brackets [SO4], hydrates CuSO4·5H2O supported.",
            ty.small
        ),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
