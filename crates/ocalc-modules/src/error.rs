//! Module errors.

use thiserror::Error;

/// Errors from module registration / dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModuleError {
    /// A module with the same id is already registered.
    #[error("module '{0}' is already registered")]
    AlreadyRegistered(String),
    /// No module provides the requested function/unit.
    #[error("no module provides '{0}'")]
    NotFound(String),
    /// Module rejected initialisation.
    #[error("module '{id}' failed to initialise: {reason}")]
    InitFailed {
        /// Module id.
        id: String,
        /// Reason.
        reason: String,
    },
    /// Underlying provider returned an error string.
    #[error("module error: {0}")]
    Provider(String),
    /// Module metadata failed validation (Phase 3 hardening).
    #[error("invalid module metadata for '{id}': {reason}")]
    InvalidMetadata {
        /// Module id (or `<unknown>` when the id itself is invalid).
        id: String,
        /// Human-readable reason.
        reason: String,
    },
    /// Module version is incompatible with the host engine API.
    #[error(
        "module '{id}' version {version} is incompatible with engine API {engine_api}: {reason}"
    )]
    IncompatibleVersion {
        /// Module id.
        id: String,
        /// Module version.
        version: String,
        /// Host engine API version.
        engine_api: String,
        /// Reason.
        reason: String,
    },
    /// Module source is not trusted (unsigned external/repository module).
    ///
    /// Reserved for future external/signed/ODS-repository modules: built-in
    /// modules never produce this error. Hosts that load third-party plugins
    /// should call [`crate::ModuleMetadata::verify_signature`] before
    /// registration and refuse [`Self::UntrustedSource`] modules.
    #[error("module '{id}' from {origin} is not trusted: {reason}")]
    UntrustedSource {
        /// Module id.
        id: String,
        /// Human-readable source description.
        origin: String,
        /// Human-readable reason.
        reason: String,
    },
}
