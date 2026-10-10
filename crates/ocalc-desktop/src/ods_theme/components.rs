//! Shared component styles: accent/danger/subtle buttons with explicit
//! hover, pressed and disabled states (button feedback + focus visibility).

use iced::widget::button::{Status, Style};
use iced::{Background, Border, Color, Theme};

fn base(accent: Color, text: Color, status: Status) -> Style {
    let dim = Color { a: 0.55, ..accent };
    let faint = Color { a: 0.22, ..accent };
    let background = match status {
        Status::Active => Some(Background::Color(faint)),
        Status::Hovered => Some(Background::Color(dim)),
        Status::Pressed => Some(Background::Color(accent)),
        Status::Disabled => None,
    };
    Style {
        background,
        text_color: text,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: accent,
        },
        ..Style::default()
    }
}

/// Primary action style (evaluate, active mode tab, accent buttons).
pub fn primary_button(accent: Color) -> impl Fn(&Theme, Status) -> Style {
    move |_theme, status| base(accent, Color::WHITE, status)
}

/// Destructive action style (clear history, delete entry, MC).
pub fn danger_button(danger: Color) -> impl Fn(&Theme, Status) -> Style {
    move |_theme, status| base(danger, Color::WHITE, status)
}

/// Quiet style for keypad digits and toolbar buttons: visible hover border
/// (keyboard-focus affordance) without chromatic noise.
pub fn subtle_button() -> impl Fn(&Theme, Status) -> Style {
    move |theme: &Theme, status: Status| {
        let extended = theme.extended_palette();
        let (bg, border_color) = match status {
            Status::Active => (None, Color::TRANSPARENT),
            Status::Hovered => (
                Some(Background::Color(extended.background.weak.color)),
                extended.background.strong.color,
            ),
            Status::Pressed => (
                Some(Background::Color(extended.background.strong.color)),
                extended.background.strong.color,
            ),
            Status::Disabled => (None, Color::TRANSPARENT),
        };
        Style {
            background: bg,
            text_color: theme.palette().text,
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..Style::default()
        }
    }
}
