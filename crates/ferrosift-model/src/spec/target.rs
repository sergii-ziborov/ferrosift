//! Execution targets, host capabilities, classifications, and streaming modes.

use alloc::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// A compilation and execution target supported by an operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The current native host target.
    Native,
    /// Browser-compatible `wasm32-unknown-unknown`.
    Wasm32UnknownUnknown,
    /// Statically linked Linux using musl.
    LinuxMusl,
    /// OpenWrt-class Linux systems.
    OpenWrt,
    /// Constrained embedded systems with an allocator.
    Embedded,
}

/// A deterministic set of supported targets.
pub type TargetSet = BTreeSet<Target>;

/// An explicit host effect an operation may request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostCapability {
    /// Read access to explicitly granted files.
    FilesystemRead,
    /// Write access to explicitly granted files.
    FilesystemWrite,
    /// Access to explicitly granted network endpoints.
    Network,
    /// Read access to explicitly granted environment values.
    Environment,
    /// Access to a host-provided clock.
    Clock,
    /// Access to a host-provided random source.
    Randomness,
}

/// A deterministic set of required host capabilities.
pub type CapabilitySet = BTreeSet<HostCapability>;

/// A review classification independent of host capability requirements.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClassification {
    /// Output can depend on randomness.
    Random,
    /// Behavior depends on an external system.
    External,
    /// Behavior exists primarily for compatibility with a legacy format.
    Legacy,
    /// The operation requires an explicit unsafe-use policy decision.
    Unsafe,
}

/// A deterministic set of operation review classifications.
pub type ClassificationSet = BTreeSet<OperationClassification>;

/// How an operation's output size relates to its input size.
///
/// The execution budget bounds growth with an expansion ratio, which is the
/// right instrument only when output is a function of input. An operation that
/// generates from its arguments has no meaningful ratio against an empty
/// input, and one that reduces to a fixed-size digest has no meaningful
/// growth at all. Saying which of the three an operation is lets the executor
/// apply the limit that fits instead of the one that happens to be there.
///
/// This is a declaration, not an exemption: every variant is still bounded by
/// the budget's absolute output limit and by cancellation. Only the *ratio*
/// changes, and only for the variant that cannot have one.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OutputBehavior {
    /// Output grows with input: encodings, compression, substitution.
    ///
    /// The default, because it is the honest assumption for anything that has
    /// not said otherwise.
    #[default]
    InputProportional,
    /// Output is determined by the arguments, not the input.
    ///
    /// Sequence and identifier generators. The expansion ratio is not applied,
    /// because the input it would divide by is unrelated to the result; the
    /// operation is expected to bound itself against the output limit.
    InputIndependent,
    /// Output is a bounded summary regardless of input size.
    ///
    /// Hashes, checksums, and statistics. Behaves like the proportional case
    /// today; naming it separately is what will later let the executor skip
    /// growth accounting it cannot need.
    Reducer,
}

/// The streaming contract implemented by an operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamingSupport {
    /// Only whole-value execution is supported.
    Unsupported,
    /// The operation buffers the complete value internally.
    Buffered,
    /// The operation can process bounded chunks incrementally.
    Incremental,
}
