//! Independent evidence records for operation claims.

use alloc::{collections::BTreeMap, string::String};

use serde::{Deserialize, Serialize};

use super::Target;

/// Lifecycle state of one evidence claim.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    /// No evidence has been collected.
    Missing,
    /// Evidence collection is explicitly planned.
    Planned,
    /// The referenced evidence passed its gate.
    Passed,
    /// The referenced evidence failed its gate.
    Failed,
}

/// One evidence state and its optional stable reference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceRecord {
    /// Current evidence state.
    pub state: EvidenceState,
    /// Repository-relative fixture, report, license, or provenance reference.
    pub reference: Option<String>,
}

impl EvidenceRecord {
    /// Returns whether state and reference form a valid record.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        match self.state {
            EvidenceState::Missing => self.reference.is_none(),
            EvidenceState::Planned => self.reference.as_deref().is_none_or(is_non_empty),
            EvidenceState::Passed | EvidenceState::Failed => {
                self.reference.as_deref().is_some_and(is_non_empty)
            }
        }
    }

    /// Returns whether the record proves its claim.
    #[must_use]
    pub fn is_verified(&self) -> bool {
        self.state == EvidenceState::Passed && self.reference.as_deref().is_some_and(is_non_empty)
    }
}

/// Evidence dimensions required for a reviewable operation catalog.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceSummary {
    /// Source and implementation provenance.
    pub provenance: EvidenceRecord,
    /// License review evidence.
    pub license: EvidenceRecord,
    /// Behavioral conformance evidence.
    pub conformance: EvidenceRecord,
    /// Performance measurement state.
    pub benchmark: EvidenceRecord,
    /// Per-target compilation and conformance evidence.
    pub target_checks: BTreeMap<Target, EvidenceRecord>,
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}
