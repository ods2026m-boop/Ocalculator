//! Capability declarations: what a module may do.

use serde::{Deserialize, Serialize};

/// Fine-grained capability advertised by a module.
///
/// The engine (or ODS-os host) can filter modules by capability, e.g. only
/// allow [`Capability::Pure`] modules in a sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Pure computation, no I/O.
    Pure,
    /// Trigonometric / elementary functions.
    Trigonometry,
    /// Statistical functions.
    Statistics,
    /// Unit conversion.
    Units,
    /// History / storage access.
    History,
    /// Graph / sampling support.
    Graphing,
    /// Arbitrary extension point.
    Custom(String),
}

impl Capability {
    /// Human-readable identifier.
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            Self::Pure => "pure".to_owned(),
            Self::Trigonometry => "trigonometry".to_owned(),
            Self::Statistics => "statistics".to_owned(),
            Self::Units => "units".to_owned(),
            Self::History => "history".to_owned(),
            Self::Graphing => "graphing".to_owned(),
            Self::Custom(s) => format!("custom:{s}"),
        }
    }
}
