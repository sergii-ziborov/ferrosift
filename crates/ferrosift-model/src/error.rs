//! Typed failures produced by model validation and value access.

use alloc::string::String;
use core::fmt;

use crate::ValueKind;

/// A stable validation failure in the portable model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelError {
    /// An operation identifier does not follow the canonical grammar.
    InvalidOperationId,
    /// A step identifier does not follow the canonical grammar.
    InvalidStepId,
    /// An argument carries a kind outside the closed portable algebra.
    UnknownArgumentKind,
    /// More than one recipe step carries the same stable identifier.
    DuplicateStepId {
        /// The duplicated step identifier.
        id: String,
    },
}

impl ModelError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidOperationId => "model.operation_id.invalid",
            Self::InvalidStepId => "model.step_id.invalid",
            Self::UnknownArgumentKind => "model.argument.unknown_kind",
            Self::DuplicateStepId { .. } => "model.recipe.duplicate_step_id",
        }
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateStepId { id } => write!(formatter, "{}: {id}", self.code()),
            _ => formatter.write_str(self.code()),
        }
    }
}

impl core::error::Error for ModelError {}

/// An explicit value access or conversion failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueError {
    /// The value carries a different representation than the caller requested.
    TypeMismatch {
        /// Representation required by the caller.
        expected: ValueKind,
        /// Representation carried by the value.
        actual: ValueKind,
    },
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch { expected, actual } => {
                write!(formatter, "expected {expected}, found {actual}")
            }
        }
    }
}

impl core::error::Error for ValueError {}
