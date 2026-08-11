//! Stable operation-specification validation failures.

use alloc::string::String;
use core::fmt;

use super::Target;

/// A machine-readable operation-specification validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpecError {
    /// A required string or set is empty.
    InvalidField {
        /// Stable field path.
        field: &'static str,
    },
    /// Two argument declarations use the same name.
    DuplicateArgument {
        /// Duplicated argument name.
        name: String,
    },
    /// An argument default carries a different representation.
    InvalidArgumentDefault {
        /// Invalid argument name.
        name: String,
    },
    /// An evidence state/reference pair is malformed.
    InvalidEvidenceRecord {
        /// Stable evidence field path.
        field: &'static str,
    },
    /// A required evidence dimension is not verified.
    MissingEvidence {
        /// Stable evidence field path.
        field: &'static str,
    },
    /// A declared target has no verified target record.
    MissingTargetEvidence {
        /// Target lacking evidence.
        target: Target,
    },
}

impl SpecError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidField { .. } => "model.operation_spec.field_invalid",
            Self::DuplicateArgument { .. } => "model.operation_spec.argument_duplicate",
            Self::InvalidArgumentDefault { .. } => "model.operation_spec.argument_default_invalid",
            Self::InvalidEvidenceRecord { .. } => "model.operation_spec.evidence_invalid",
            Self::MissingEvidence { .. } => "model.operation_spec.evidence_missing",
            Self::MissingTargetEvidence { .. } => "model.operation_spec.target_evidence_missing",
        }
    }
}

impl fmt::Display for SpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidField { field }
            | Self::InvalidEvidenceRecord { field }
            | Self::MissingEvidence { field } => write!(formatter, "{}: {field}", self.code()),
            Self::DuplicateArgument { name } | Self::InvalidArgumentDefault { name } => {
                write!(formatter, "{}: {name}", self.code())
            }
            Self::MissingTargetEvidence { target } => {
                write!(formatter, "{}: {target:?}", self.code())
            }
        }
    }
}

impl core::error::Error for SpecError {}
