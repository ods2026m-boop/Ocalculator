//! Finance view: compound-interest, loan, growth and percentage
//! calculators stacked in one mode.
//!
//! Math runs in `ocalc-finance` through `commands`; this view only edits
//! field text. Rates are entered as human percentages (`5` = 5 %).

use iced::widget::{column, scrollable, text};
use iced::{Element, Length};

use crate::messages::Message;
use crate::state::AppState;
use crate::views::{hint, labeled_field, section_card};

/// Result line: value text, danger-colored error, or muted placeholder.
///
/// Phase 8: consistent error/empty presentation across all four calculators.
fn outcome<'a>(
    value: Option<String>,
    error: &'a Option<String>,
    small: f32,
    danger: iced::Color,
    muted: iced::Color,
) -> Element<'a, Message> {
    match (value, error) {
        (Some(v), _) => text(format!("= {v}")).size(small).into(),
        (None, Some(err)) => {
            let first = err.lines().next().unwrap_or("error");
            text(format!("⚠ {first}")).size(small).color(danger).into()
        }
        (None, None) => text("—").size(small).color(muted).into(),
    }
}

/// Finance mode: four stacked calculators.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let ty = state.theme.scaled_typography();
    let sp = state.theme.spacing;
    let f = &state.finance;
    let danger = state.theme.colors.danger;
    let muted = state.theme.colors.muted;

    // Compound interest: future value + effective-rate readout.
    let compound = section_card(
        "Compound interest",
        column![
            labeled_field(
                "Principal",
                &f.compound_p,
                Some("fin-compound-p"),
                move |s| Message::FinanceCompoundChanged(0, s),
                ty.small
            ),
            labeled_field(
                "Rate (%/period)",
                &f.compound_r,
                None,
                move |s| Message::FinanceCompoundChanged(1, s),
                ty.small
            ),
            labeled_field(
                "Periods",
                &f.compound_n,
                None,
                move |s| Message::FinanceCompoundChanged(2, s),
                ty.small
            ),
            outcome(
                f.compound_result.map(|v| state.fmt_money(v)),
                &f.compound_error,
                ty.small,
                danger,
                muted
            ),
            labeled_field(
                "Nominal (%/yr)",
                &f.compound_nominal,
                None,
                move |s| Message::FinanceNominalChanged(0, s),
                ty.small
            ),
            labeled_field(
                "Compounds/yr",
                &f.compound_m,
                None,
                move |s| Message::FinanceNominalChanged(1, s),
                ty.small
            ),
            outcome(
                f.compound_effective
                    .map(|v| format!("{} % effective", state.fmt_pct(v))),
                &f.compound_effective_error,
                ty.small,
                danger,
                muted
            ),
        ]
        .spacing(4)
        .into(),
        ty,
        sp,
    );

    // Loans: monthly payment + lifetime totals.
    let loan_lines = match (f.loan_payment, f.loan_total, f.loan_interest) {
        (Some(pmt), Some(total), Some(interest)) => format!(
            "= {} /mo · {} total · {} interest",
            state.fmt_money(pmt),
            state.fmt_money(total),
            state.fmt_money(interest)
        ),
        _ => String::new(),
    };
    let loan = section_card(
        "Loan",
        column![
            labeled_field(
                "Principal",
                &f.loan_p,
                None,
                move |s| Message::FinanceLoanChanged(0, s),
                ty.small
            ),
            labeled_field(
                "APR (%)",
                &f.loan_apr,
                None,
                move |s| Message::FinanceLoanChanged(1, s),
                ty.small
            ),
            labeled_field(
                "Months",
                &f.loan_n,
                None,
                move |s| Message::FinanceLoanChanged(2, s),
                ty.small
            ),
            outcome(
                if loan_lines.is_empty() {
                    None
                } else {
                    Some(loan_lines)
                },
                &f.loan_error,
                ty.small,
                danger,
                muted
            ),
        ]
        .spacing(4)
        .into(),
        ty,
        sp,
    );

    // Growth: end value from rate, and rate from endpoints.
    let growth = section_card(
        "Growth",
        column![
            labeled_field(
                "Start",
                &f.growth_start,
                None,
                move |s| Message::FinanceGrowthChanged(0, s),
                ty.small
            ),
            labeled_field(
                "Rate (%)",
                &f.growth_rate,
                None,
                move |s| Message::FinanceGrowthChanged(1, s),
                ty.small
            ),
            labeled_field(
                "Periods",
                &f.growth_n,
                None,
                move |s| Message::FinanceGrowthChanged(2, s),
                ty.small
            ),
            outcome(
                f.growth_result.map(|v| state.fmt_money(v)),
                &f.growth_error,
                ty.small,
                danger,
                muted
            ),
            labeled_field(
                "Rate from: start",
                &f.rate_start,
                None,
                move |s| Message::FinanceRateChanged(0, s),
                ty.small
            ),
            labeled_field(
                "end",
                &f.rate_end,
                None,
                move |s| Message::FinanceRateChanged(1, s),
                ty.small
            ),
            labeled_field(
                "periods",
                &f.rate_n,
                None,
                move |s| Message::FinanceRateChanged(2, s),
                ty.small
            ),
            outcome(
                f.rate_result
                    .map(|v| format!("{} %/period", state.fmt_pct(v))),
                &f.rate_error,
                ty.small,
                danger,
                muted
            ),
        ]
        .spacing(4)
        .into(),
        ty,
        sp,
    );

    // Percentages: share of base + change between two values.
    let percent = section_card(
        "Percentages",
        column![
            labeled_field(
                "Base",
                &f.percent_base,
                None,
                move |s| Message::FinancePercentChanged(0, s),
                ty.small
            ),
            labeled_field(
                "Percent",
                &f.percent_pct,
                None,
                move |s| Message::FinancePercentChanged(1, s),
                ty.small
            ),
            outcome(
                f.percent_result.map(|v| state.fmt_money(v)),
                &f.percent_error,
                ty.small,
                danger,
                muted
            ),
            labeled_field(
                "Old value",
                &f.change_old,
                None,
                move |s| Message::FinanceChangeChanged(0, s),
                ty.small
            ),
            labeled_field(
                "New value",
                &f.change_new,
                None,
                move |s| Message::FinanceChangeChanged(1, s),
                ty.small
            ),
            outcome(
                f.change_result.map(|v| format!("{} %", state.fmt_pct(v))),
                &f.change_error,
                ty.small,
                danger,
                muted
            ),
        ]
        .spacing(4)
        .into(),
        ty,
        sp,
    );

    let body = column![compound, loan, growth, percent,]
        .spacing(sp.sm)
        .width(Length::Fill);

    column![
        text("Finance").size(ty.heading),
        scrollable(body).height(Length::Fill),
        hint("Rates are percentages: 5 means 5 %.", ty.small),
    ]
    .spacing(sp.sm)
    .width(Length::Fill)
    .into()
}
