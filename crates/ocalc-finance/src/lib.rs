//! `ocalc-finance`: finance toolkit for Ocalculator.
//!
//! Percentage, compound-interest, loan-amortization and growth kernels as
//! native Rust functions, bridged into engine expressions by
//! [`FinanceModule`] (`percent()`, `compound()`, `loan_payment()`, …).
//! Rates are decimal fractions (`0.05` = 5 %); percentages are `0–100`
//! human units (`percent(200, 15) = 30`).

mod growth;
mod interest;
mod loan;
mod module;
mod percent;

pub use growth::{growth, growth_rate};
pub use interest::{compound, effective_rate};
pub use loan::{loan_interest, loan_payment, loan_total};
pub use module::{FinanceModule, FinanceProvider};
pub use percent::{percent, percent_change};
