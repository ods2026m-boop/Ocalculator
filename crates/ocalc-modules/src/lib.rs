//! `ocalc-modules`: plugin architecture.
//!
//! New functionality is added **without modifying the core engine** by
//! implementing [`CalculatorModule`] (or the narrower [`FunctionProvider`] /
//! [`UnitProvider`]) and registering it with [`ModuleRegistry`].
//!
//! ```rust
//! use ocalc_modules::{CalculatorModule, Capability, FunctionProvider, ModuleMetadata};
//! use ocalc_number::Number;
//!
//! struct Double;
//! impl FunctionProvider for Double {
//!     fn function_names(&self) -> Vec<String> { vec!["double".into()] }
//!     fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>> {
//!         if name == "double" && args.len() == 1 {
//!             Some(args[0].add(&args[0]).map_err(|e| e.to_string()))
//!         } else { None }
//!     }
//! }
//! impl CalculatorModule for Double {
//!     fn metadata(&self) -> ModuleMetadata {
//!         ModuleMetadata::new("demo-double", "1.0.0", "doubles a value")
//!             .with_capability(Capability::Custom("demo".into()))
//!     }
//! }
//! ```

mod capability;
mod error;
mod manager;
mod traits;

pub use capability::Capability;
pub use error::ModuleError;
pub use manager::ModuleRegistry;
pub use traits::{
    CalculatorModule, FunctionProvider, ModuleMetadata, ModuleSource, UnitProvider,
    ENGINE_PLUGIN_API_VERSION, MODULE_METADATA_FORMAT_VERSION,
};
