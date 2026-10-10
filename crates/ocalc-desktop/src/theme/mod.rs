//! Compatibility re-exports over [`crate::ods_theme`].
//!
//! New code should import from `crate::ods_theme` directly.

#![allow(unused_imports)]

pub mod ods;

pub use crate::ods_theme::{OdsTheme, ThemeMode, Typography};
