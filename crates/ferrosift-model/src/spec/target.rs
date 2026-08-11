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
