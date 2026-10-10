//! `ocalc-desktop`: Iced desktop frontend for the stable OCalc Engine.
//!
//! All calculation flows through `ocalc_engine` public APIs (`commands.rs`);
//! this binary only wires [`OcalcApp`] into the Iced runtime.

mod app;
mod commands;
mod messages;
mod modules;
mod ods_theme;
mod state;
mod theme;
mod views;

use app::OcalcApp;

/// Desktop entry point.
///
/// Production boot (dirs, migration, startup log) happens in
/// [`OcalcApp::new`]. A fatal Iced runtime error is logged to the production
/// log as well as stderr — never a silent exit.
pub fn main() -> iced::Result {
    let result = iced::application(OcalcApp::new, OcalcApp::update, OcalcApp::view)
        .title("Ocalculator")
        .subscription(OcalcApp::subscription)
        .theme(OcalcApp::theme)
        .window_size(iced::Size::new(1100.0, 720.0))
        .run();
    if let Err(ref e) = result {
        let msg = format!("fatal UI error: {e:?}");
        eprintln!("error: {msg}");
        ocalc_history::log_event("ui", &msg);
    }
    result
}
