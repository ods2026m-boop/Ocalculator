//! Converter view: category selector + unit pickers + result.
//!
//! Categories and conversions come from the engine's unit tables
//! (`commands::convert_units`); speed composes length/time conversions with
//! engine-evaluated arithmetic. This view only edits text and selections.

use iced::widget::{button, column, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::commands;
use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::{AppState, UnitCategoryId};

/// Converter mode.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let c = &state.converter;

    // Category selector.
    let mut cats = row![text("Category:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(6);
    for cat in UnitCategoryId::ALL {
        let label = if c.category == cat {
            format!("[{}]", cat.label())
        } else {
            cat.label().to_owned()
        };
        cats = cats.push(
            button(text(label).size(ty.small))
                .on_press(Message::ConverterCategoryChanged(cat))
                .style(subtle_button()),
        );
    }

    let value_box = text_input("Value, e.g. 1.5", &c.value_text)
        .on_input(Message::ConverterValueChanged)
        .on_submit(Message::Nop)
        .id("conv-value")
        .size(ty.body)
        .padding(10);

    let pickers: Element<'_, Message> = if c.category == UnitCategoryId::Speed {
        speed_pickers(state)
    } else {
        let symbols = c.category.symbols();
        let from_pick = pick_list(symbols.clone(), Some(c.from.clone()), move |s| {
            Message::ConverterUnitChanged(0, s)
        })
        .placeholder("from");
        let to_pick = pick_list(symbols, Some(c.to.clone()), move |s| {
            Message::ConverterUnitChanged(1, s)
        })
        .placeholder("to");
        row![
            from_pick,
            button(text("⇄").size(ty.body)).on_press(Message::ConverterSwap),
            to_pick,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
    };

    let danger = state.theme.colors.danger;
    let muted = state.theme.colors.muted;
    let result: Element<'_, Message> = match (c.result, &c.error) {
        (Some(v), _) => {
            let unit = if c.category == UnitCategoryId::Speed {
                format!("{}/{}", c.to, c.speed_to_time)
            } else {
                c.to.clone()
            };
            text(format!("= {v} {unit}"))
                .size(ty.display * 0.875)
                .into()
        }
        (None, Some(err)) => {
            let first = err.lines().next().unwrap_or("conversion failed");
            column![
                text(format!("⚠ {first}")).size(ty.small).color(danger),
                text("Check the value is numeric and the units are compatible.")
                    .size(ty.tiny)
                    .color(muted),
            ]
            .spacing(2)
            .into()
        }
        (None, None) => text("Enter a value to convert it.")
            .size(ty.body)
            .color(muted)
            .into(),
    };

    column![
        text(format!("{} converter", c.category.label())).size(ty.heading),
        cats,
        value_box,
        pickers,
        result,
        text("Conversions use the engine's built-in tables.").size(ty.small),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Speed pickers: length/time pairs on both sides.
fn speed_pickers(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let c = &state.converter;
    let lens = commands::speed_length_symbols();
    let times = commands::speed_time_symbols();
    row![
        column![
            text("From:").size(ty.small),
            pick_list(lens.clone(), Some(c.from.clone()), move |s| {
                Message::ConverterUnitChanged(0, s)
            }),
            pick_list(times.clone(), Some(c.speed_from_time.clone()), move |s| {
                Message::ConverterTimeUnitChanged(0, s)
            }),
        ]
        .spacing(4),
        button(text("⇄").size(ty.body)).on_press(Message::ConverterSwap),
        column![
            text("To:").size(ty.small),
            pick_list(lens, Some(c.to.clone()), move |s| {
                Message::ConverterUnitChanged(1, s)
            }),
            pick_list(times, Some(c.speed_to_time.clone()), move |s| {
                Message::ConverterTimeUnitChanged(1, s)
            }),
        ]
        .spacing(4),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}
