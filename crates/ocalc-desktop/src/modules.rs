//! Scientific module discovery for the desktop frontend.
//!
//! The [`OCalc Engine`](ocalc_engine) stays untouched: this module only
//! *discovers* the independent scientific crates (`ocalc-cas`,
//! `ocalc-physics`, `ocalc-chemistry`, `ocalc-finance`) and registers them
//! through the stable [`CalculatorEngine`](ocalc_engine::CalculatorEngine)
//! plugin API (`register_module` / `unregister`). No core code paths change.
//!
//! UI responsibilities (kept minimal, no redesign):
//! - [`discover_scientific_modules`]: full inventory for the settings panel.
//! - [`register_scientific_modules`]: one-shot startup wiring.
//! - [`create_module`]: factory for re-enabling a module by id.
//! - [`search_functions`]: case-insensitive substring search over
//!   `engine.function_names()`.

use std::sync::Arc;

use ocalc_engine::{CalculatorEngine, CalculatorModule};

/// Ids of the Phase 6 scientific modules (sorted).
pub const SCIENTIFIC_MODULE_IDS: [&str; 4] = ["cas", "chemistry", "finance", "physics"];

/// Build one fresh instance of every scientific module.
///
/// Each call returns new `Arc`s — cheap, stateless providers — so hosts can
/// call this at startup *and* to re-enable a single module later via
/// [`create_module`].
#[must_use]
pub fn discover_scientific_modules() -> Vec<Arc<dyn CalculatorModule>> {
    vec![
        Arc::new(ocalc_cas::CasModule::new()),
        Arc::new(ocalc_chemistry::ChemistryModule::new()),
        Arc::new(ocalc_finance::FinanceModule::new()),
        Arc::new(ocalc_physics::PhysicsModule::new()),
    ]
}

/// Build a single scientific module by id (`None` for unknown ids).
#[must_use]
pub fn create_module(id: &str) -> Option<Arc<dyn CalculatorModule>> {
    match id {
        "cas" => Some(Arc::new(ocalc_cas::CasModule::new())),
        "chemistry" => Some(Arc::new(ocalc_chemistry::ChemistryModule::new())),
        "finance" => Some(Arc::new(ocalc_finance::FinanceModule::new())),
        "physics" => Some(Arc::new(ocalc_physics::PhysicsModule::new())),
        _ => None,
    }
}

/// Register every scientific module with `engine`.
///
/// Version-checked via [`ocalc_engine::ENGINE_PLUGIN_API_VERSION`]:
/// duplicates are skipped (idempotent, so calling twice — e.g. tests +
/// startup — is harmless) and real failures are reported loudly through the
/// production log + stderr instead of being swallowed.
pub fn register_scientific_modules(engine: &mut CalculatorEngine) {
    for m in discover_scientific_modules() {
        let id = m.name();
        match engine.register_module_checked(m, ocalc_engine::ENGINE_PLUGIN_API_VERSION) {
            Ok(()) => {}
            Err(ocalc_modules::ModuleError::AlreadyRegistered(_)) => {}
            Err(e) => {
                let msg = format!("module '{id}' failed to load: {e}");
                eprintln!("warning: {msg}");
                ocalc_history::log_event("module", &msg);
            }
        }
    }
}

/// Per-module load report for startup logging and diagnostics.
///
/// Returns `(loaded_ids, failures)` where failures are human-readable
/// `module '<id>': <reason>` strings. Never panics.
#[must_use]
pub fn module_load_report(engine: &CalculatorEngine) -> (Vec<String>, Vec<String>) {
    use ocalc_engine::ENGINE_PLUGIN_API_VERSION;
    let mut loaded = Vec::new();
    let mut failures = Vec::new();
    for id in SCIENTIFIC_MODULE_IDS {
        if engine.module_ids().contains(&id.to_owned()) {
            loaded.push(id.to_owned());
        } else {
            failures.push(format!("module '{id}' is not registered"));
        }
    }
    // Compatibility check against the running engine API.
    for m in discover_scientific_modules() {
        let meta = m.metadata();
        if let Err(e) = meta.check_compatibility(ENGINE_PLUGIN_API_VERSION) {
            failures.push(format!("module '{}': {e}", meta.id));
        }
    }
    loaded.sort();
    (loaded, failures)
}

/// One-line description for a scientific module id (for the settings panel).
#[must_use]
pub fn module_description(id: &str) -> &'static str {
    match id {
        "cas" => "computer algebra: simplify, calculus, solving",
        "chemistry" => "periodic table, formula parser, molar masses",
        "finance" => "percent, compound interest, loans, growth",
        "physics" => "constants, mechanics, electricity (SI)",
        _ => "unknown module",
    }
}

/// Case-insensitive substring search over all engine functions
/// (built-ins + registered modules). Empty query returns everything.
/// Results are sorted (they come sorted from the engine).
#[must_use]
pub fn search_functions(engine: &CalculatorEngine, query: &str) -> Vec<String> {
    let q = query.trim().to_ascii_lowercase();
    let names = engine.function_names();
    if q.is_empty() {
        return names;
    }
    names
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().contains(&q))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocalc_engine::CalculatorEngine;

    #[test]
    fn discovery_covers_all_ids() {
        let mods = discover_scientific_modules();
        let mut ids: Vec<String> = mods.iter().map(|m| m.name()).collect();
        ids.sort();
        assert_eq!(ids, vec!["cas", "chemistry", "finance", "physics"]);
        for m in &mods {
            m.metadata().validate().unwrap();
        }
    }

    #[test]
    fn factory_roundtrip() {
        for id in SCIENTIFIC_MODULE_IDS {
            let m = create_module(id).unwrap_or_else(|| panic!("no factory for {id}"));
            assert_eq!(m.name(), id);
        }
        assert!(create_module("nosuch").is_none());
        assert!(!module_description("cas").is_empty());
    }

    #[test]
    fn registration_is_idempotent_and_searchable() {
        let mut e = CalculatorEngine::new();
        register_scientific_modules(&mut e);
        for id in SCIENTIFIC_MODULE_IDS {
            assert!(
                e.module_ids().contains(&id.to_owned()),
                "{:?}",
                e.module_ids()
            );
        }
        // Second pass must not error the host (duplicates skipped).
        register_scientific_modules(&mut e);
        // Function search finds scientific kernels alongside built-ins.
        assert!(search_functions(&e, "force").contains(&"force".to_owned()));
        assert!(search_functions(&e, "FORCE").contains(&"force".to_owned()));
        assert!(search_functions(&e, "molar_mass").contains(&"molar_mass".to_owned()));
        assert!(search_functions(&e, "compound").contains(&"compound".to_owned()));
        assert!(search_functions(&e, "solve_linear").contains(&"solve_linear".to_owned()));
        assert!(search_functions(&e, "nosuchfunction_xyz").is_empty());
        assert!(!search_functions(&e, "").is_empty());
    }

    #[test]
    fn scientific_functions_evaluate() {
        let mut e = CalculatorEngine::new();
        register_scientific_modules(&mut e);
        assert!((e.eval("force(2, 3)").unwrap().to_f64() - 6.0).abs() < 1e-12);
        assert!((e.eval("compound(1000, 0.05, 1)").unwrap().to_f64() - 1050.0).abs() < 1e-9);
        assert!((e.eval("percent(200, 15)").unwrap().to_f64() - 30.0).abs() < 1e-12);
        assert!((e.eval("molar_mass(1, 2, 8, 1)").unwrap().to_f64() - 18.015).abs() < 1e-9);
        assert!((e.eval("solve_linear(2, -4)").unwrap().to_f64() - 2.0).abs() < 1e-12);
    }
}
