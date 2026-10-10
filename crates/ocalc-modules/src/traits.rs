//! Core plugin traits.

use serde::{Deserialize, Serialize};

use ocalc_number::Number;

use crate::{Capability, ModuleError};

/// Engine plugin API version that modules are validated against.
///
/// Tracks the workspace version (currently `1.0.0`). All built-in modules are
/// versioned with `CARGO_PKG_VERSION`, so they pass strict `register_checked`
/// today. It bumps only on breaking [`CalculatorModule`] / dispatch changes.
/// Modules with a different *major* version are rejected by
/// [`ModuleMetadata::check_compatibility`] unless the host opts out.
pub const ENGINE_PLUGIN_API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Serialized form version for [`ModuleMetadata`] JSON (Phase 3, v1).
pub const MODULE_METADATA_FORMAT_VERSION: u32 = 1;

/// Where a module comes from. Reserved for future external / signed /
/// ODS-repository modules; today every built-in module is [`Self::Builtin`].
///
/// Serialized with `snake_case` tags. Defaults to [`Self::Builtin`] so
/// pre-1.0 metadata JSON (which has no `source` key) keeps loading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModuleSource {
    /// Compiled into the host binary (all current modules).
    #[default]
    Builtin,
    /// Third-party plugin loaded from a filesystem path.
    External {
        /// Filesystem path the module was loaded from.
        path: String,
    },
    /// Module fetched from an ODS package repository.
    Repository {
        /// Repository URL the module was fetched from.
        url: String,
    },
}

impl ModuleSource {
    /// Short human-readable description (used in error messages and logs).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Builtin => "built-in".to_owned(),
            Self::External { path } => format!("external file {path:?}"),
            Self::Repository { url } => format!("ODS repository {url:?}"),
        }
    }

    /// True for modules compiled into the host binary.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::Builtin)
    }
}

/// Static metadata every module must expose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Unique id, e.g. `"ocalc-stats"`. Kebab-case, 1–64 chars.
    pub id: String,
    /// Semver version string (`major.minor.patch`, prerelease allowed).
    pub version: String,
    /// One-line description (1–256 chars).
    pub description: String,
    /// Declared capabilities (non-empty, deduplicated).
    pub capabilities: Vec<Capability>,
    /// Serialization format version (always [`MODULE_METADATA_FORMAT_VERSION`]).
    #[serde(default = "default_metadata_format_version")]
    pub format_version: u32,
    /// Where the module comes from (future external/signed/ODS-repository
    /// support). Missing in pre-1.0 JSON → [`ModuleSource::Builtin`].
    #[serde(default)]
    pub source: ModuleSource,
    /// Detached trust signature for external/repository modules.
    ///
    /// `None` today (all modules are built-in). Future ODS-repository modules
    /// will carry a signature here; see [`ModuleMetadata::verify_signature`].
    #[serde(default)]
    pub signature: Option<String>,
}

fn default_metadata_format_version() -> u32 {
    MODULE_METADATA_FORMAT_VERSION
}

impl ModuleMetadata {
    /// Minimal metadata; chain `.with_capability(..)` to add more.
    #[must_use]
    pub fn new(id: &str, version: &str, description: &str) -> Self {
        Self {
            id: id.to_owned(),
            version: version.to_owned(),
            description: description.to_owned(),
            capabilities: vec![Capability::Pure],
            format_version: MODULE_METADATA_FORMAT_VERSION,
            source: ModuleSource::Builtin,
            signature: None,
        }
    }

    /// Builder-style capability append.
    #[must_use]
    pub fn with_capability(mut self, cap: Capability) -> Self {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
        self
    }

    /// Validate id / version / description / capabilities.
    ///
    /// # Errors
    /// [`ModuleError::InvalidMetadata`] describing the first violation.
    pub fn validate(&self) -> Result<(), ModuleError> {
        let id = self.id.trim();
        if id.is_empty() {
            return Err(ModuleError::InvalidMetadata {
                id: "<unknown>".to_owned(),
                reason: "module id must not be empty".to_owned(),
            });
        }
        if id.len() > 64 {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: "module id must be <= 64 chars".to_owned(),
            });
        }
        let id_ok = id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && id
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && !id.contains("--")
            && !id.starts_with('-')
            && !id.ends_with('-');
        if !id_ok {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason:
                    "module id must be kebab-case [a-z0-9-] (no leading/trailing/double dashes)"
                        .to_owned(),
            });
        }
        parse_semver(&self.version).map_err(|reason| ModuleError::InvalidMetadata {
            id: self.id.clone(),
            reason: format!("invalid semver version {:?}: {reason}", self.version),
        })?;
        let desc = self.description.trim();
        if desc.is_empty() {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: "description must not be empty".to_owned(),
            });
        }
        if self.description.len() > 256 {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: "description must be <= 256 chars".to_owned(),
            });
        }
        if self.capabilities.is_empty() {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: "at least one capability is required".to_owned(),
            });
        }
        if self.format_version != MODULE_METADATA_FORMAT_VERSION {
            return Err(ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: format!(
                    "unsupported metadata format version {}, expected {}",
                    self.format_version, MODULE_METADATA_FORMAT_VERSION
                ),
            });
        }
        Ok(())
    }

    /// Check module compatibility against an engine API version.
    ///
    /// Policy (v1): major versions must match; a module *older* minor than the
    /// engine is accepted, a module *newer* minor than the engine is rejected
    /// (it may rely on newer host features). Prerelease versions never match
    /// stable ones.
    ///
    /// # Errors
    /// [`ModuleError::IncompatibleVersion`] on mismatch, or
    /// [`ModuleError::InvalidMetadata`] when either version is not semver.
    pub fn check_compatibility(&self, engine_api: &str) -> Result<(), ModuleError> {
        let (m_major, m_minor, _, m_pre) =
            parse_semver(&self.version).map_err(|reason| ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: format!("invalid semver version {:?}: {reason}", self.version),
            })?;
        let (e_major, e_minor, _, e_pre) =
            parse_semver(engine_api).map_err(|reason| ModuleError::InvalidMetadata {
                id: self.id.clone(),
                reason: format!("invalid engine version {engine_api:?}: {reason}"),
            })?;
        // Prerelease modules are only compatible with the exact same prerelease engine.
        if !m_pre.is_empty() || !e_pre.is_empty() {
            if self.version != engine_api {
                return Err(ModuleError::IncompatibleVersion {
                    id: self.id.clone(),
                    version: self.version.clone(),
                    engine_api: engine_api.to_owned(),
                    reason: "prerelease versions require an exact match".to_owned(),
                });
            }
            return Ok(());
        }
        if m_major != e_major {
            return Err(ModuleError::IncompatibleVersion {
                id: self.id.clone(),
                version: self.version.clone(),
                engine_api: engine_api.to_owned(),
                reason: format!("major version mismatch ({m_major} != {e_major})"),
            });
        }
        if m_minor > e_minor {
            return Err(ModuleError::IncompatibleVersion {
                id: self.id.clone(),
                version: self.version.clone(),
                engine_api: engine_api.to_owned(),
                reason: format!("module requires newer engine API (minor {m_minor} > {e_minor})"),
            });
        }
        Ok(())
    }

    /// Verify trust for external / repository modules (future ODS-repository
    /// support).
    ///
    /// Policy for v1.0.0:
    /// - [`ModuleSource::Builtin`] modules are always trusted.
    /// - [`ModuleSource::External`] / [`ModuleSource::Repository`] modules
    ///   must carry a non-empty detached `signature`; otherwise an
    ///   [`ModuleError::UntrustedSource`] error is returned with a clear
    ///   remediation hint.
    ///
    /// Cryptographic signature verification itself is intentionally out of
    /// scope for v1 (no network / keyring yet); this hook reserves the API
    /// so ODS-os hosts can enforce `verify_signature` before
    /// `register_checked` without API breakage.
    ///
    /// # Errors
    /// [`ModuleError::UntrustedSource`] when a non-builtin module has no
    /// signature.
    pub fn verify_signature(&self) -> Result<(), ModuleError> {
        match &self.source {
            ModuleSource::Builtin => Ok(()),
            external => {
                let sig_ok = self
                    .signature
                    .as_deref()
                    .is_some_and(|s| !s.trim().is_empty());
                if sig_ok {
                    Ok(())
                } else {
                    Err(ModuleError::UntrustedSource {
                        id: self.id.clone(),
                        origin: external.describe(),
                        reason: "missing detached signature; sign the module or allow unsigned plugins explicitly".to_owned(),
                    })
                }
            }
        }
    }

    /// Serialize with a version envelope (`{"format_version":1,"metadata":{…}}`).
    ///
    /// # Errors
    /// JSON serialization failures (cannot happen for valid metadata).
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&serde_json::json!({
            "format_version": MODULE_METADATA_FORMAT_VERSION,
            "metadata": self,
        }))
        .map_err(|e| e.to_string())
    }

    /// Deserialize [`Self::to_json`] output (accepts both enveloped and bare
    /// metadata objects for backward compatibility).
    ///
    /// # Errors
    /// JSON parse errors or [`ModuleError`] validation failures as strings.
    pub fn from_json(s: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| format!("invalid metadata JSON: {e}"))?;
        let inner = v.get("metadata").unwrap_or(&v);
        let meta: Self =
            serde_json::from_value(inner.clone()).map_err(|e| format!("bad metadata: {e}"))?;
        meta.validate().map_err(|e| e.to_string())?;
        Ok(meta)
    }
}

/// Parse `major.minor.patch[-pre]` into parts. Build metadata (`+…`) is ignored.
fn parse_semver(v: &str) -> Result<(u64, u64, u64, String), String> {
    let v = v.trim();
    if v.is_empty() {
        return Err("empty version".to_owned());
    }
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, p.to_owned()),
        None => (v, String::new()),
    };
    // Strip build metadata.
    let core = core.split('+').next().unwrap_or(core);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("expected major.minor.patch, got {v:?}"));
    }
    let mut nums = [0u64; 3];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("non-numeric component {p:?} in {v:?}"));
        }
        nums[i] = p
            .parse::<u64>()
            .map_err(|_| format!("bad number {p:?} in {v:?}"))?;
    }
    if !pre.is_empty()
        && !pre
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(format!("bad prerelease {pre:?}"));
    }
    Ok((nums[0], nums[1], nums[2], pre))
}

/// Provides one or more callable functions.
pub trait FunctionProvider: Send + Sync {
    /// Lowercase function names this provider implements.
    fn function_names(&self) -> Vec<String>;

    /// Cheap membership test used by the registry hot path.
    ///
    /// The default implementation scans [`Self::function_names`] (one
    /// allocation). High-cardinality providers should override this to avoid
    /// the `Vec` allocation per call.
    fn provides(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        self.function_names()
            .iter()
            .any(|n| n.to_ascii_lowercase() == key)
    }

    /// Attempt to evaluate `name(args)`.
    ///
    /// Return `None` when `name` is not handled by this provider so the
    /// registry can try the next one. Return `Some(Err(..))` for a real
    /// evaluation failure.
    fn call(&self, name: &str, args: &[Number]) -> Option<Result<Number, String>>;
}

/// Provides unit conversions.
pub trait UnitProvider: Send + Sync {
    /// Unit symbols handled, e.g. `["m", "km", "ft"]`.
    fn unit_names(&self) -> Vec<String>;

    /// Cheap membership test (default scans [`Self::unit_names`]).
    fn provides(&self, symbol: &str) -> bool {
        let key = symbol.to_ascii_lowercase();
        self.unit_names()
            .iter()
            .any(|n| n.to_ascii_lowercase() == key)
    }

    /// Convert `value` from `from` to `to`. `None` = not handled.
    fn convert(&self, value: f64, from: &str, to: &str) -> Option<Result<f64, String>>;
}

/// A full calculator module. Implementors may also implement
/// [`FunctionProvider`] and/or [`UnitProvider`]; the blanket dispatch in
/// [`crate::ModuleRegistry`] downcasts via these optional hooks.
///
/// The required surface is intentionally tiny — everything else has a default
/// derived from [`metadata`](CalculatorModule::metadata), so the engine (and
/// third-party hosts) can treat every module uniformly by `name`/`version`/
/// `functions`/`capabilities` without downcasting.
pub trait CalculatorModule: Send + Sync {
    /// Static metadata.
    fn metadata(&self) -> ModuleMetadata;

    /// Module id (`metadata().id`). Provided for ergonomic dispatch.
    fn name(&self) -> String {
        self.metadata().id
    }

    /// Module version (`metadata().version`).
    fn version(&self) -> String {
        self.metadata().version
    }

    /// Declared capabilities (`metadata().capabilities`).
    fn capabilities(&self) -> Vec<Capability> {
        self.metadata().capabilities
    }

    /// Lowercase function names served via [`as_function_provider`](CalculatorModule::as_function_provider)
    /// (empty when the module provides no functions).
    fn functions(&self) -> Vec<String> {
        self.as_function_provider()
            .map(|p| p.function_names())
            .unwrap_or_default()
    }

    /// Called once on registration; default is a no-op success.
    fn init(&self) -> Result<(), String> {
        Ok(())
    }

    /// Function-provider hook. Default: this module provides no functions.
    fn as_function_provider(&self) -> Option<&dyn FunctionProvider> {
        None
    }

    /// Unit-provider hook. Default: this module provides no units.
    fn as_unit_provider(&self) -> Option<&dyn UnitProvider> {
        None
    }
}
