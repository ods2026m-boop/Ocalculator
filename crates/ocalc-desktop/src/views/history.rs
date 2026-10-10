//! History sidebar: favorites shelf, search, filter, reuse, per-item
//! delete, clear.
//!
//! All persistence comes from the public [`ocalc_history`] API; per-item
//! delete and capacity caps rebuild through `commands::rebuild_history`
//! (ids preserved). Favorites live in [`AppState`](crate::state::AppState)
//! and persist through settings.

use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

use crate::messages::Message;
use crate::ods_theme::{danger_button, subtle_button};
use crate::state::AppState;
use crate::views::{empty_state, heading};

/// History sidebar (fixed width, collapsible via the header toggle).
///
/// Phase 8: improved visualization — timestamps on entries, result-first
/// rows with star/delete actions, a "showing newest N" hint, and a
/// favorites shelf with its own clear action.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let muted = state.theme.colors.muted;
    let danger = state.theme.colors.danger;

    let header = row![
        text(format!("History ({})", state.history.len())).size(ty.body),
        iced::widget::Space::new().width(Length::Fill),
        button(text("Clear").size(ty.small))
            .on_press(Message::HistoryClear)
            .style(danger_button(danger)),
    ]
    .align_y(Alignment::Center)
    .spacing(sp.sm);

    let search_box = text_input("Search history… e.g. sin", &state.history_search)
        .on_input(Message::HistorySearchChanged)
        .size(ty.small)
        .padding(sp.xs);

    // Favorites shelf (starred calculations, newest last).
    let fav_header = row![
        heading(format!("★ Favorites ({})", state.favorites.len()), ty.small),
        iced::widget::Space::new().width(Length::Fill),
        button(text("Clear").size(ty.tiny))
            .on_press(Message::FavoritesClear)
            .style(subtle_button()),
    ]
    .align_y(Alignment::Center)
    .spacing(sp.xs);
    let mut favs = column![fav_header].spacing(sp.xs);
    if state.favorites.is_empty() {
        favs = favs.push(empty_state(
            "No favorites yet — press ☆ Star on any result to pin it here.",
            ty.tiny,
            muted,
        ));
    } else {
        for f in &state.favorites {
            let id = f.id;
            favs = favs.push(
                row![
                    button(text(format!("{} = {}", f.expression, f.result)).size(ty.tiny))
                        .on_press(Message::FavoriteReuse(id))
                        .style(subtle_button())
                        .width(Length::Fill),
                    button(text("✕").size(ty.tiny))
                        .on_press(Message::FavoriteDelete(id))
                        .style(danger_button(danger)),
                ]
                .spacing(sp.xs)
                .align_y(Alignment::Center),
            );
        }
    }

    // Calculation log: expression button (tap to reuse) plus result line
    // with timestamp, star toggle and delete.
    let entries = state.filtered_history();
    let total = entries.len();
    let mut list = column![].spacing(sp.xs);
    if entries.is_empty() {
        let msg: String = if state.history_search.trim().is_empty() {
            "No history yet — evaluated expressions appear here.\nTip: press Enter to evaluate, then click an entry to reuse it.".to_owned()
        } else {
            format!(
                "No matches for {:?}.\nTry a shorter search or clear the filter.",
                state.history_search.trim()
            )
        };
        list = list.push(empty_state(msg, ty.small, muted));
    } else {
        for e in entries.iter().take(50) {
            let id = e.id;
            let star = if state.is_favorited(&e.expression) {
                "★"
            } else {
                "☆"
            };
            let when = e.timestamp.format("%H:%M").to_string();
            let entry_row = column![
                button(text(e.expression.clone()).size(ty.small))
                    .on_press(Message::HistoryReuse(id))
                    .style(subtle_button())
                    .width(Length::Fill),
                row![
                    text(format!("#{} · {when} = {}", e.id, e.result))
                        .size(ty.tiny)
                        .color(muted)
                        .width(Length::Fill),
                    button(text(star).size(ty.tiny))
                        .on_press(Message::FavoriteToggleEntry(id))
                        .style(subtle_button()),
                    button(text("✕").size(ty.tiny))
                        .on_press(Message::HistoryDelete(id))
                        .style(danger_button(danger)),
                ]
                .spacing(sp.xs)
                .align_y(Alignment::Center),
            ]
            .spacing(2);
            list = list.push(entry_row);
        }
        if total > 50 {
            list = list.push(
                text(format!(
                    "Showing newest 50 of {total} — refine the search to find older entries."
                ))
                .size(ty.tiny)
                .color(muted)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
            );
        } else if !state.history_search.trim().is_empty() {
            list = list.push(
                text(format!(
                    "{total} match{} — click any entry to reuse it.",
                    if total == 1 { "" } else { "es" }
                ))
                .size(ty.tiny)
                .color(muted)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
            );
        }
    }
    let scrolled = scrollable(list).height(Length::Fill);

    column![header, search_box, favs, scrolled]
        .spacing(sp.sm)
        .width(Length::Fixed(300.0))
        .height(Length::Fill)
        .into()
}
