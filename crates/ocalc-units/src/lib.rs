//! `ocalc-units`: unit conversion system.
//!
//! Linear units convert through a `factor_to_base`; temperature uses an
//! affine transform. Custom units can be registered at runtime without
//! touching this crate.

mod converter;
mod custom;
mod error;
mod module;
mod tables;

pub use converter::{UnitCategory, UnitConverter, UnitDefinition};
pub use custom::CustomUnitRegistry;
pub use error::UnitError;
pub use module::UnitsModule;
pub use tables::builtin_converter;
