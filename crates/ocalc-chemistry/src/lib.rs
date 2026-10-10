//! `ocalc-chemistry`: chemistry toolkit for Ocalculator.
//!
//! Periodic table data ([`table`]), chemical formula parsing
//! ([`parse_formula`] — `H2O`, `CO2`, `NaCl`, `Ca(OH)2`, hydrates), molar
//! masses ([`molecular_mass`]) and per-element [`composition`], bridged into
//! engine expressions by [`ChemistryModule`] (`element_mass()`,
//! `molar_mass()` — numeric `(atomic number, count)` pairs, since engine
//! calls cannot carry formula strings).

mod formula;
mod mass;
mod module;
pub mod table;

pub use formula::{parse_formula, Composition};
pub use mass::{composition, molar_mass_pairs, molecular_mass, Component};
pub use module::{ChemistryModule, ChemistryProvider};
