//! Compatibility names attached to operation specifications.

use alloc::string::String;

use serde::{Deserialize, Serialize};

/// A naming and behavior profile understood by the compatibility layer.
///
/// One variant per reference *version*, not one for the project. A
/// compatibility claim is against a version: when upstream changes an
/// operation, the honest record is that `FerroSift` matches one profile and
/// differs from the next, which cannot be said at all with a single variant.
///
/// Adding a newer profile never retires an older one. A caller pinned to 11.3
/// is entitled to know `FerroSift` still matches it, and the evidence for that
/// keeps its own fixtures. Where an operation's semantics genuinely changed
/// between versions, the versioned operation identifier carries the other half
/// — the changed behaviour becomes `@2` rather than silently replacing `@1`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityProfile {
    /// `FerroSift`'s native naming profile.
    Native,
    /// Names and observable semantics from `CyberChef` 11.3.
    CyberChefV11_3,
    /// Names and observable semantics from `CyberChef` 11.4.
    CyberChefV11_4,
}

/// A profile-scoped compatibility name for an operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompatibilityAlias {
    /// Profile in which the alias is meaningful.
    pub profile: CompatibilityProfile,
    /// Exact operation name used by that profile.
    pub name: String,
}
