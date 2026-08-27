//! Compatibility names attached to operation specifications.

use alloc::string::String;
use alloc::vec::Vec;

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

impl CompatibilityProfile {
    /// Every `CyberChef` profile, in release order.
    ///
    /// Ordered because an operation that upstream *introduced* in some version
    /// is aliased in that version and in everything after it, and the catalog
    /// should not have to spell that out one version at a time. Deriving the
    /// rest from this list is what lets a specification say when its name
    /// starts existing rather than asserting it always did.
    ///
    /// A new variant belongs here as well as above. Leaving it out does not
    /// silently under-claim: the replay gate for that profile has no aliases to
    /// check and says so.
    pub const CYBERCHEF: &'static [Self] = &[Self::CyberChefV11_3, Self::CyberChefV11_4];

    /// Whether this profile names a `CyberChef` release rather than `FerroSift`
    /// itself.
    ///
    /// Written as an exhaustive match so that adding a variant is a compile
    /// error here, next to the list it also has to join.
    #[must_use]
    pub const fn is_cyberchef(self) -> bool {
        match self {
            Self::Native => false,
            Self::CyberChefV11_3 | Self::CyberChefV11_4 => true,
        }
    }
}

/// A profile-scoped compatibility name for an operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompatibilityAlias {
    /// Profile in which the alias is meaningful.
    pub profile: CompatibilityProfile,
    /// Exact operation name used by that profile.
    pub name: String,
}

impl CompatibilityAlias {
    /// One alias per `CyberChef` profile from `earliest` onward.
    ///
    /// An operation the reference has always had starts at the oldest profile
    /// and is claimed in all of them. One the reference *introduced* starts
    /// where it was introduced, because the versions before that cannot answer
    /// to the name and saying they can would be saying something false about
    /// the reference rather than about this port.
    ///
    /// Later profiles are included rather than enumerated, so a name that
    /// exists from some version onward does not have to be re-declared each
    /// time a newer reference is added. That is not a way to claim a profile
    /// without evidence: each profile's replay gate refuses an alias no
    /// replayed case of that profile backs, so a newly added profile makes
    /// these claims fail until its fixtures exist.
    ///
    /// A profile that is not a `CyberChef` release yields nothing.
    #[must_use]
    pub fn cyberchef_since(earliest: CompatibilityProfile, name: &str) -> Vec<Self> {
        if !earliest.is_cyberchef() {
            return Vec::new();
        }
        CompatibilityProfile::CYBERCHEF
            .iter()
            .filter(|profile| **profile >= earliest)
            .map(|profile| Self {
                profile: *profile,
                name: String::from(name),
            })
            .collect()
    }
}
