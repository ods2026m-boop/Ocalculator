//! Back-compat shim: the ODS system lives in [`crate::ods_theme`].
//!
//! Kept (and compiled) for the documented `theme/` structure; the names
//! below are re-exported for old import paths. Allowed unused: a shim is
//! only referenced by out-of-tree code.
//!
//! New code should import from `crate::ods_theme` directly.

#![allow(unused_imports)]

pub use crate::ods_theme::{OdsTheme, ThemeMode, Typography};
