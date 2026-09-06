//! Hosted job and artifact layer for `FerroSift` adapters.
//!
//! Portable operations never receive filesystem or network handles. This crate
//! is the place those capabilities are introduced under explicit policy: an
//! allowlisted open, an opaque artifact handle, and a shared execution report
//! that CLI and MCP can serialize the same way.

#![forbid(unsafe_code)]

mod allowlist;
mod artifact;
mod error;
mod report;
mod service;

pub use allowlist::PathAllowlist;
pub use artifact::{ArtifactId, ArtifactMeta, ArtifactStore, StoreConfig};
pub use error::{HostError, HostResult};
pub use report::{
    ExecutionReport, InspectReport, ReportStatus, SearchHit, StepLocationReport, TraceEventReport,
    ValueSummaryReport,
};
pub use service::{
    HostConfig, HostService, InputKind, InspectRequest, OpenRequest, RecipeFormat, RunRequest,
};
