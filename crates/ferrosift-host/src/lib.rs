//! Hosted job and artifact layer for `FerroSift` adapters.
//!
//! Portable operations never receive filesystem or network handles. This crate
//! is the place those capabilities are introduced under explicit policy: an
//! allowlisted open, an opaque artifact handle, and a shared execution report
//! that CLI and MCP can serialize the same way.

#![forbid(unsafe_code)]

mod allowlist;
mod artifact;
mod candidates;
mod error;
mod report;
mod repro;
mod service;

pub use allowlist::PathAllowlist;
pub use artifact::{ArtifactId, ArtifactMeta, ArtifactStore, StoreConfig};
pub use candidates::{
    MAX_CANDIDATES, CandidateCheck, CandidateRecipe, CandidateResult, CandidatesReport,
    CandidatesRequest, CheckOutcome, evaluate_checks, parse_candidate_format, validate_batch,
};
pub use error::{HostError, HostResult};
pub use report::{
    ExecutionReport, InspectReport, ReportStatus, SearchHit, StepLocationReport, TraceEventReport,
    ValueSummaryReport,
};
pub use repro::{
    BuildPackageRequest, ExpectedOrigin, ReproCheckReport, ReproManifest, ReproPackage,
    build_package, compare_observation, ensure_export_allowed, parse_input_kind,
    parse_recipe_format, read_package, write_package,
};
pub use service::{
    ExportReproRequest, HostConfig, HostService, InputKind, InspectRequest, OpenRequest,
    RecipeFormat, RunRequest,
};
