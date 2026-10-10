//! Programmer view: base switcher + bit view + shifts + signed/unsigned.
//!
//! Readouts come from `commands::format_bases(last_result)`; bit toggles
//! rebuild a binary literal evaluated through the engine. No base math lives
//! here.

use iced::widget::{button, checkbox, column, row, text};
use iced::{Alignment, Element, Length};

use crate::commands;
use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::{AppState, NumBase};
use crate::views::{accent_key, display_panel, heading, hint, key};

/// Number of interactive bits (low 16 of the current integer result).
const BIT_COUNT: u32 = 16;

/// Programmer mode: base selector, readouts, bit view, keypad.
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
    let prog = &state.programmer;

    // Base switcher.
    let mut bases = row![text("Base:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(6);
    for base in NumBase::ALL {
        let label = if prog.base == base {
            format!("[{}]", base.label())
        } else {
            base.label().to_owned()
        };
        bases = bases.push(
            button(text(label).size(ty.small))
                .on_press(Message::ProgrammerBaseChanged(base))
                .style(subtle_button()),
        );
    }

    // Readouts; the selected base leads.
    let bases_fmt = match &state.last_result {
        Some(v) => commands::format_bases(v),
        None => commands::BaseFormats {
            dec: "—".to_owned(),
            hex: "—".to_owned(),
            oct: "—".to_owned(),
            bin: "—".to_owned(),
            is_integer: false,
        },
    };
    let ordered = match prog.base {
        NumBase::Bin => [
            ("BIN", &bases_fmt.bin),
            ("OCT", &bases_fmt.oct),
            ("DEC", &bases_fmt.dec),
            ("HEX", &bases_fmt.hex),
        ],
        NumBase::Oct => [
            ("OCT", &bases_fmt.oct),
            ("DEC", &bases_fmt.dec),
            ("HEX", &bases_fmt.hex),
            ("BIN", &bases_fmt.bin),
        ],
        NumBase::Dec => [
            ("DEC", &bases_fmt.dec),
            ("HEX", &bases_fmt.hex),
            ("OCT", &bases_fmt.oct),
            ("BIN", &bases_fmt.bin),
        ],
        NumBase::Hex => [
            ("HEX", &bases_fmt.hex),
            ("DEC", &bases_fmt.dec),
            ("OCT", &bases_fmt.oct),
            ("BIN", &bases_fmt.bin),
        ],
    };
    let mut readout = column![].spacing(2);
    for (name, val) in ordered {
        readout = readout.push(text(format!("{name}  {val}")).size(ty.small));
    }
    readout = readout.push(signed_line(state));

    // Bit view: interactive low-16 bits of integer results.
    let mut bits_section = column![row![
        heading("Bits (low 16)", ty.small),
        iced::widget::Space::new().width(Length::Fill),
        button(text(if prog.show_bits { "Hide" } else { "Show" }).size(ty.small))
            .on_press(Message::ProgrammerToggleBits)
            .style(subtle_button()),
    ]
    .align_y(Alignment::Center),]
    .spacing(sp.xs);
    if prog.show_bits {
        let has_int = state
            .last_result
            .as_ref()
            .is_some_and(|v| v.as_bigint().is_ok());
        if has_int {
            bits_section = bits_section.push(bit_row(state, 15, 8));
            bits_section = bits_section.push(bit_row(state, 7, 0));
        } else {
            bits_section = bits_section.push(
                text("Evaluate an integer first — then tap bits to flip them.")
                    .size(ty.tiny)
                    .color(state.theme.colors.muted),
            );
        }
    }

    let pad = column![
        row![
            key("A", Message::AppendInput("A".into())),
            key("B", Message::AppendInput("B".into())),
            key("C", Message::AppendInput("C".into())),
            key("D", Message::AppendInput("D".into())),
            key("E", Message::AppendInput("E".into())),
            key("F", Message::AppendInput("F".into())),
        ]
        .spacing(sp.xs),
        row![
            key("AND", Message::AppendInput(" AND ".into())),
            key("OR", Message::AppendInput(" OR ".into())),
            key("XOR", Message::AppendInput(" XOR ".into())),
            key("NOT", Message::AppendInput("NOT ".into())),
            key("MOD", Message::AppendInput(" MOD ".into())),
        ]
        .spacing(sp.xs),
        row![
            key("<<", Message::AppendInput(" << ".into())),
            key(">>", Message::AppendInput(" >> ".into())),
            key("&", Message::AppendInput(" & ".into())),
            key("|", Message::AppendInput(" | ".into())),
            key("~", Message::AppendInput("~".into())),
        ]
        .spacing(sp.xs),
        row![
            key("7", Message::AppendInput("7".into())),
            key("8", Message::AppendInput("8".into())),
            key("9", Message::AppendInput("9".into())),
            key("0x", Message::AppendInput("0x".into())),
            key("C", Message::Clear),
        ]
        .spacing(sp.xs),
        row![
            key("4", Message::AppendInput("4".into())),
            key("5", Message::AppendInput("5".into())),
            key("6", Message::AppendInput("6".into())),
            key("0b", Message::AppendInput("0b".into())),
            key("CE", Message::ClearEntry),
        ]
        .spacing(sp.xs),
        row![
            key("1", Message::AppendInput("1".into())),
            key("2", Message::AppendInput("2".into())),
            key("3", Message::AppendInput("3".into())),
            key("0o", Message::AppendInput("0o".into())),
            accent_key("=", Message::Evaluate, state.theme.colors.accent),
        ]
        .spacing(sp.xs),
        row![
            key("0", Message::AppendInput("0".into())),
            key("(", Message::AppendInput("(".into())),
            key(")", Message::AppendInput(")".into())),
            key("+", Message::AppendInput("+".into())),
            key("-", Message::AppendInput("-".into())),
        ]
        .spacing(sp.xs),
    ]
    .spacing(sp.xs);

    column![
        display,
        bases,
        readout,
        bits_section,
        pad,
        hint(
            "Bit toggles rebuild a binary literal evaluated by the engine.",
            ty.small
        ),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Signed/unsigned 64-bit interpretation line (display formatting only).
fn signed_line(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let text_line = match &state.last_result {
        Some(v) => match v.as_i64() {
            Some(i) => {
                let u = i as u64;
                if state.programmer.signed {
                    format!("INT64  {i}   (unsigned: {u})")
                } else {
                    format!("UINT64  {u}   (signed: {i})")
                }
            }
            None => "INT64  —  (needs a 64-bit integer result)".to_owned(),
        },
        None => "INT64  —".to_owned(),
    };
    row![
        text(text_line).size(ty.small).width(Length::Fill),
        checkbox(state.programmer.signed)
            .label("Signed")
            .on_toggle(Message::ProgrammerSignedChanged),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .into()
}

/// One row of bit toggles, `hi..=lo` (descending).
fn bit_row(state: &AppState, hi: u32, lo: u32) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let bits = state.last_result.as_ref().and_then(|v| v.as_bigint().ok());
    let mut r = row![].spacing(2);
    for i in (lo..=hi.min(BIT_COUNT - 1)).rev() {
        let set = bits.as_ref().is_some_and(|b| {
            use num_traits::Zero;
            (b >> (i as usize)) & num_bigint::BigInt::from(1) != num_bigint::BigInt::zero()
        });
        let label = if set { "1" } else { "0" };
        r = r.push(
            button(text(label).size(ty.tiny))
                .on_press(Message::ProgrammerToggleBit(i))
                .style(subtle_button())
                .width(Length::Fill),
        );
    }
    r.into()
}
