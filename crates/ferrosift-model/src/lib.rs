//! Portable data-model primitives for `FerroSift` recipes and operations.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use core::fmt;

mod argument;
mod error;
mod operation_id;
mod recipe;
mod spec;
mod step_id;
mod decimal;
mod value;

pub use argument::{ArgumentValue, Arguments};
pub use error::{ModelError, ValueError};
pub use operation_id::OperationId;
pub use recipe::{Recipe, RecipeMetadata, RecipeStep};
pub use spec::{
    ArgumentKind, ArgumentSpec, CapabilitySet, ClassificationSet, CompatibilityAlias,
    CompatibilityProfile, EvidenceRecord, EvidenceState, EvidenceSummary, HostCapability,
    OperationClassification, OperationSpec, OutputBehavior, SpecError, StreamingSupport, Target,
    TargetSet, ValueConstraint,
};
pub use step_id::StepId;
pub use decimal::{DecimalSpecial, DecimalValue};
pub use value::{
    NumberValue, StructuredValue, TextEncoding, TextValue, Value, ValueKind, VirtualFile,
};

/// Version of `FerroSift`'s serialized recipe and value schema.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// Current schema version emitted by this crate.
    pub const CURRENT: Self = Self(1);

    /// Creates an explicit schema version.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }

    /// Returns the numeric schema version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}
