//! Compatibility names attached to operation specifications.

use alloc::string::String;

use serde::{Deserialize, Serialize};

/// A naming and behavior profile understood by the compatibility layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityProfile {
    /// `FerroSift`'s native naming profile.
    Native,
    /// Names and observable semantics from `CyberChef` 11.3.
    CyberChefV11_3,
}

/// A profile-scoped compatibility name for an operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompatibilityAlias {
    /// Profile in which the alias is meaningful.
    pub profile: CompatibilityProfile,
    /// Exact operation name used by that profile.
    pub name: String,
}
