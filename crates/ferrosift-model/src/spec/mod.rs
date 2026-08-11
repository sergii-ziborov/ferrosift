//! Machine-readable contracts for operation discovery and validation.

use alloc::{collections::BTreeSet, string::String, vec::Vec};

use serde::{Deserialize, Serialize};

use crate::OperationId;

mod argument;
mod compatibility;
mod error;
mod evidence;
mod target;

pub use argument::{ArgumentKind, ArgumentSpec, ValueConstraint};
pub use compatibility::{CompatibilityAlias, CompatibilityProfile};
pub use error::SpecError;
pub use evidence::{EvidenceRecord, EvidenceState, EvidenceSummary};
pub use target::{
    CapabilitySet, ClassificationSet, HostCapability, OperationClassification, StreamingSupport,
    Target, TargetSet,
};

/// A complete machine-readable operation contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationSpec {
    /// Stable versioned operation identifier.
    pub id: OperationId,
    /// Human-facing operation name.
    pub display_name: String,
    /// Human-facing category.
    pub category: String,
    /// Human-facing description.
    pub description: String,
    /// Names used by explicit compatibility profiles.
    pub aliases: Vec<CompatibilityAlias>,
    /// Accepted input representation.
    pub input: ValueConstraint,
    /// Produced output representation.
    pub output: ValueConstraint,
    /// Typed argument declarations.
    pub arguments: Vec<ArgumentSpec>,
    /// Supported compilation and execution targets.
    pub targets: TargetSet,
    /// Explicit host effects required by the operation.
    pub capabilities: CapabilitySet,
    /// Independent review classifications.
    pub classifications: ClassificationSet,
    /// Whether identical inputs and arguments produce identical output.
    pub deterministic: bool,
    /// Streaming contract.
    pub streaming: StreamingSupport,
    /// Optional inverse operation contract.
    pub inverse: Option<OperationId>,
    /// Independent evidence dimensions.
    pub evidence: EvidenceSummary,
}

impl OperationSpec {
    /// Validates cross-field operation contract invariants.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] for the first deterministic validation failure.
    pub fn validate(&self) -> Result<(), SpecError> {
        validate_text("display_name", &self.display_name)?;
        validate_text("category", &self.category)?;
        validate_text("description", &self.description)?;
        validate_constraint("input", &self.input)?;
        validate_constraint("output", &self.output)?;
        if self.targets.is_empty() {
            return Err(SpecError::InvalidField { field: "targets" });
        }

        for alias in &self.aliases {
            validate_text("aliases.name", &alias.name)?;
        }

        let mut argument_names = BTreeSet::new();
        for argument in &self.arguments {
            validate_text("arguments.name", &argument.name)?;
            validate_text("arguments.description", &argument.description)?;
            if !argument_names.insert(argument.name.as_str()) {
                return Err(SpecError::DuplicateArgument {
                    name: argument.name.clone(),
                });
            }
            if argument
                .default
                .as_ref()
                .is_some_and(|value| !argument.kind.matches(value))
            {
                return Err(SpecError::InvalidArgumentDefault {
                    name: argument.name.clone(),
                });
            }
        }

        validate_evidence_record("evidence.provenance", &self.evidence.provenance)?;
        validate_evidence_record("evidence.license", &self.evidence.license)?;
        validate_evidence_record("evidence.conformance", &self.evidence.conformance)?;
        validate_evidence_record("evidence.benchmark", &self.evidence.benchmark)?;
        for record in self.evidence.target_checks.values() {
            validate_evidence_record("evidence.target_checks", record)?;
        }

        require_evidence("evidence.provenance", &self.evidence.provenance)?;
        require_evidence("evidence.license", &self.evidence.license)?;
        require_evidence("evidence.conformance", &self.evidence.conformance)?;
        for target in &self.targets {
            if !self
                .evidence
                .target_checks
                .get(target)
                .is_some_and(EvidenceRecord::is_verified)
            {
                return Err(SpecError::MissingTargetEvidence { target: *target });
            }
        }
        Ok(())
    }
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SpecError> {
    if value.trim().is_empty() {
        Err(SpecError::InvalidField { field })
    } else {
        Ok(())
    }
}

fn validate_constraint(field: &'static str, constraint: &ValueConstraint) -> Result<(), SpecError> {
    if matches!(constraint, ValueConstraint::OneOf(values) if values.is_empty()) {
        Err(SpecError::InvalidField { field })
    } else {
        Ok(())
    }
}

fn validate_evidence_record(field: &'static str, record: &EvidenceRecord) -> Result<(), SpecError> {
    if record.is_structurally_valid() {
        Ok(())
    } else {
        Err(SpecError::InvalidEvidenceRecord { field })
    }
}

fn require_evidence(field: &'static str, record: &EvidenceRecord) -> Result<(), SpecError> {
    if record.is_verified() {
        Ok(())
    } else {
        Err(SpecError::MissingEvidence { field })
    }
}
