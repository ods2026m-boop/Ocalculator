//! Scientific view: trig/hyperbolic/log/power keypad + constants panel +
//! function panel + RAD/DEG/GRAD selector.
//!
//! Buttons append engine syntax verbatim; the selector drives
//! [`AppState::set_angle_unit`](crate::state::AppState).

use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::subtle_button;
use crate::state::{AngleUnit, AppState};
use crate::views::{accent_key, display_panel, heading, hint, key};

/// Function buttons: (label, literal).
const FUNCTIONS: [(&str, &str); 16] = [
    ("sin", "sin("),
    ("cos", "cos("),
    ("tan", "tan("),
    ("asin", "asin("),
    ("acos", "acos("),
    ("atan", "atan("),
    ("sinh", "sinh("),
    ("cosh", "cosh("),
    ("tanh", "tanh("),
    ("log", "log("),
    ("ln", "ln("),
    ("√", "sqrt("),
    ("∛", "cbrt("),
    ("xʸ", "^"),
    ("x!", "!"),
    ("eˣ", "exp("),
];

/// Constants: (label, literal).
const CONSTANTS: [(&str, &str); 6] = [
    ("π", "pi"),
    ("e", "e"),
    ("τ", "tau"),
    ("φ", "phi"),
    ("∞", "inf"),
    ("NaN", "nan"),
];

/// Scientific mode: angle selector, function grid, constants, keypad.
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

    // RAD/DEG/GRAD selector.
    let mut angle = row![text("Angle:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(6);
    for unit in AngleUnit::ALL {
        let label = if state.angle == unit {
            format!("[{}]", unit.label())
        } else {
            unit.label().to_owned()
        };
        angle = angle.push(
            button(text(label).size(ty.small))
                .on_press(Message::AngleUnitChanged(unit))
                .style(subtle_button()),
        );
    }
    if state.angle == AngleUnit::Gradians {
        angle = angle.push(text("100 grad = 90°").size(ty.small));
    }

    // Function panel: 4×4 grid of engine functions.
    let mut fns = column![heading("Functions", ty.heading)].spacing(sp.xs);
    for chunk in FUNCTIONS.chunks(4) {
        let mut r = row![].spacing(sp.xs);
        for (label, lit) in chunk {
            r = r.push(key(label, Message::AppendInput((*lit).into())));
        }
        fns = fns.push(r);
    }

    // Constants panel.
    let mut consts = row![text("Constants:").size(ty.small)]
        .align_y(Alignment::Center)
        .spacing(6);
    for (label, lit) in CONSTANTS {
        consts = consts.push(key(label, Message::AppendInput(lit.into())));
    }

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
            key("(", Message::AppendInput("(".into())),
            key(")", Message::AppendInput(")".into())),
        ]
        .spacing(sp.xs),
    ]
    .spacing(sp.xs);

    column![
        display,
        angle,
        fns,
        consts,
        pad,
        hint(
            "Functions append with an open paren — close it before =.",
            ty.small
        ),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
