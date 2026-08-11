//! Stable failures produced while registering operations.

use alloc::string::String;
use core::fmt;

use ferrosift_model::{CompatibilityProfile, OperationId, SpecError};

/// A fail-closed operation registration error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    /// The operation's machine-readable contract is invalid.
    InvalidSpec(SpecError),
    /// The canonical operation ID is already registered.
    DuplicateOperation {
        /// Colliding canonical operation ID.
        id: OperationId,
    },
    /// An alias is already registered within the same compatibility profile.
    DuplicateAlias {
        /// Profile that owns the colliding alias.
        profile: CompatibilityProfile,
        /// Exact colliding alias text.
        name: String,
    },
}

impl RegistryError {
    /// Returns a stable machine-readable failure code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidSpec(error) => error.code(),
            Self::DuplicateOperation { .. } => "core.registry.operation_duplicate",
            Self::DuplicateAlias { .. } => "core.registry.alias_duplicate",
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => fmt::Display::fmt(error, formatter),
            Self::DuplicateOperation { id } => write!(formatter, "{}: {id}", self.code()),
            Self::DuplicateAlias { profile, name } => {
                write!(formatter, "{}: {profile:?}/{name}", self.code())
            }
        }
    }
}

impl core::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidSpec(error) => Some(error),
            Self::DuplicateOperation { .. } | Self::DuplicateAlias { .. } => None,
        }
    }
}

impl From<SpecError> for RegistryError {
    fn from(error: SpecError) -> Self {
        Self::InvalidSpec(error)
    }
}
