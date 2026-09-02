//! Independent evidence records for the claims a catalog makes.
//!
//! Evidence used to live on every [`OperationSpec`](super::OperationSpec),
//! which put the same five records in the catalog two hundred and fifty-four
//! times. Reading them showed why: not one of them was a fact about an
//! operation. Provenance named `NOTICE`, the licence named `LICENSE`, the
//! target checks named the workflow file, the benchmark said `Missing` for
//! every operation in a repository that publishes measurements — and
//! conformance named *one test file* for the whole catalog, which was false
//! for two hundred and fifty-three of them.
//!
//! They are facts about the *build*, so there is one [`EvidenceManifest`] and
//! the registry holds it. What genuinely varies per operation — how many
//! reference cases pin it, and where it diverges — is computed from the
//! fixtures and published in `docs/compatibility/ledger.md`, which is the one
//! place that can say it without a human keeping a string up to date.

use alloc::{collections::BTreeMap, string::String};

use serde::{Deserialize, Serialize};

use super::{SpecError, Target};

/// What stands behind one evidence claim.
///
/// A *policy*, not a result. These values are committed source: they travel
/// inside the crate and say which gate a claim is held to, and a file that
/// ships with the library cannot know whether the build reading it passed
/// anything. It said `Passed` until it was noticed that a working tree with a
/// red CI run produced a manifest claiming every check had passed — the string
/// was a path to a workflow, and a path is not a result.
///
/// Whether a particular revision cleared its gates is a fact about a *run*, and
/// lives where runs live: the repository's Actions history and the release it
/// produced. See `docs/releasing.md`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    /// Nothing stands behind the claim.
    Missing,
    /// A gate is intended for this claim and does not exist yet.
    Planned,
    /// A gate exists and every build is held to it.
    Enforced,
}

/// One evidence state and the gate it names.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceRecord {
    /// What stands behind the claim.
    pub state: EvidenceState,
    /// Repository-relative gate: a workflow, a fixture, a licence, a notice.
    pub reference: Option<String>,
}

impl EvidenceRecord {
    /// A record with nothing behind it.
    #[must_use]
    pub const fn missing() -> Self {
        Self {
            state: EvidenceState::Missing,
            reference: None,
        }
    }

    /// A record naming the gate every build of this claim is held to.
    #[must_use]
    pub fn enforced(reference: impl Into<String>) -> Self {
        Self {
            state: EvidenceState::Enforced,
            reference: Some(reference.into()),
        }
    }

    /// Returns whether state and reference form a valid record.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        match self.state {
            EvidenceState::Missing => self.reference.is_none(),
            EvidenceState::Planned => self.reference.as_deref().is_none_or(is_non_empty),
            EvidenceState::Enforced => self.reference.as_deref().is_some_and(is_non_empty),
        }
    }

    /// Returns whether a gate stands behind the claim and says where it is.
    ///
    /// Named for what it can answer. It used to be `is_verified`, which read as
    /// "this was checked and it passed" — a claim about a run that committed
    /// source cannot make about itself.
    #[must_use]
    pub fn is_enforced(&self) -> bool {
        self.state == EvidenceState::Enforced && self.reference.as_deref().is_some_and(is_non_empty)
    }
}

/// What a build has checked, and where to read the check.
///
/// One per catalog rather than one per operation, because every dimension here
/// is a property of the build: the same notice file, the same licence, the same
/// workflow, the same published measurements. An operation's own contribution
/// is its [`targets`](super::OperationSpec::targets) — and a registry refuses
/// an operation claiming a target this manifest does not cover, which is the
/// invariant that used to be checked once per specification against a copy of
/// this data.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceManifest {
    /// Source and implementation provenance.
    pub provenance: EvidenceRecord,
    /// License review evidence.
    pub license: EvidenceRecord,
    /// Where the per-operation conformance result is published.
    ///
    /// Deliberately a pointer rather than an answer. How many reference cases
    /// pin one operation, and where it diverges, is computed from the committed
    /// fixtures; a string copied into the catalog could only repeat it, and
    /// repeated it wrongly for years.
    pub conformance: EvidenceRecord,
    /// Where performance measurements are published.
    pub benchmark: EvidenceRecord,
    /// Per-target compilation and conformance evidence.
    pub target_checks: BTreeMap<Target, EvidenceRecord>,
}

impl EvidenceManifest {
    /// A manifest with nothing behind it.
    ///
    /// What an empty registry holds. Registering an operation against it is
    /// refused for every target the operation declares, which is the honest
    /// answer: a target claim with no gate behind it is the thing this type
    /// exists to prevent.
    #[must_use]
    pub fn unbacked() -> Self {
        Self {
            provenance: EvidenceRecord::missing(),
            license: EvidenceRecord::missing(),
            conformance: EvidenceRecord::missing(),
            benchmark: EvidenceRecord::missing(),
            target_checks: BTreeMap::new(),
        }
    }

    /// Whether a gate stands behind this build's claim to run on `target`.
    #[must_use]
    pub fn covers(&self, target: Target) -> bool {
        self.target_checks
            .get(&target)
            .is_some_and(EvidenceRecord::is_enforced)
    }

    /// Validates the manifest's own records.
    ///
    /// Provenance, licence and conformance must name a gate; a benchmark record
    /// may be absent, because a build that has measured nothing yet is a state
    /// this can describe rather than one it should refuse. Every record must be
    /// structurally consistent — an enforced record without a reference is a
    /// claim with nowhere to check it.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError::InvalidEvidenceRecord`] for a record whose state and
    /// reference disagree, and [`SpecError::MissingEvidence`] for a required
    /// dimension with nothing behind it.
    pub fn validate(&self) -> Result<(), SpecError> {
        for (field, record) in self.records() {
            if !record.is_structurally_valid() {
                return Err(SpecError::InvalidEvidenceRecord { field });
            }
        }
        for (field, record) in [
            ("evidence.provenance", &self.provenance),
            ("evidence.license", &self.license),
            ("evidence.conformance", &self.conformance),
        ] {
            if !record.is_enforced() {
                return Err(SpecError::MissingEvidence { field });
            }
        }
        Ok(())
    }

    fn records(&self) -> impl Iterator<Item = (&'static str, &EvidenceRecord)> {
        [
            ("evidence.provenance", &self.provenance),
            ("evidence.license", &self.license),
            ("evidence.conformance", &self.conformance),
            ("evidence.benchmark", &self.benchmark),
        ]
        .into_iter()
        .chain(
            self.target_checks
                .values()
                .map(|record| ("evidence.target_checks", record)),
        )
    }
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}
