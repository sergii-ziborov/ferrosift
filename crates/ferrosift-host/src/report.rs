//! Shared serializable reports for hosted execution.

use ferrosift_core::{
    ExecutionResult, ExecutionStatus, ExecutionTrace, StepLocation, TraceEvent, TraceEventKind,
    ValueSummary,
};
use ferrosift_model::{Value, ValueKind};
use serde::Serialize;

use crate::artifact::ArtifactMeta;

/// Machine-readable envelope for one recipe execution.
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionReport {
    /// Stable schema identifier for this envelope.
    pub schema: &'static str,
    /// Whether the recipe completed or paused.
    pub status: ReportStatus,
    /// Output artifact handle when execution produced a retained value.
    pub artifact_id: String,
    /// Representation of the current value.
    pub value_kind: ValueKind,
    /// Saturating logical payload size of the current value.
    pub value_size_bytes: u64,
    /// Content digest of the retained artifact.
    pub digest_sha256: String,
    /// Current value after the last completed or skipped step.
    pub value: Value,
    /// Runtime crate version that produced this report.
    pub runtime_version: &'static str,
    /// Bounded ordered execution trace.
    pub trace: Vec<TraceEventReport>,
    /// Non-fatal notes for the caller.
    pub warnings: Vec<&'static str>,
}

/// Terminal status exposed on the wire.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    /// Every executable step completed.
    Completed,
    /// Execution paused before the indicated step.
    Paused {
        /// Zero-based position of the unexecuted step.
        step_index: usize,
    },
}

/// One catalog search hit.
#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    /// Canonical operation identifier.
    pub id: String,
    /// Human-facing name.
    pub display_name: String,
    /// Human-facing category.
    pub category: String,
    /// Short description.
    pub description: String,
    /// Matching alias names, when the query hit an alias.
    pub matched_aliases: Vec<String>,
}

/// Bounded inspect projection for one artifact.
#[derive(Clone, Debug, Serialize)]
pub struct InspectReport {
    /// Stable schema identifier.
    pub schema: &'static str,
    /// Opaque handle.
    pub artifact_id: String,
    /// Value representation.
    pub value_kind: ValueKind,
    /// Logical payload size.
    pub size_bytes: u64,
    /// Content digest.
    pub digest_sha256: String,
    /// Optional byte preview, never larger than the store preview ceiling.
    pub preview_hex: Option<String>,
    /// Whether the preview was truncated.
    pub preview_truncated: bool,
    /// Selected UTF-8 text when the value is UTF-8 text and a preview was asked.
    pub preview_text: Option<String>,
}

/// One bounded trace event.
#[derive(Clone, Debug, Serialize)]
pub struct TraceEventReport {
    /// Recipe step associated with the transition.
    pub location: StepLocationReport,
    /// Bounded transition details.
    pub kind: TraceEventKindReport,
}

/// Stable location of one recipe step.
#[derive(Clone, Debug, Serialize)]
pub struct StepLocationReport {
    /// Zero-based recipe position.
    pub index: usize,
    /// Stable recipe-local step identity.
    pub step_id: String,
    /// Canonical operation identity.
    pub operation: String,
}

/// Bounded information emitted for one execution transition.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TraceEventKindReport {
    /// An enabled operation is about to run.
    StepStarted {
        /// Input representation and size.
        input: ValueSummaryReport,
    },
    /// A disabled operation preserved its input.
    StepSkipped {
        /// Preserved value representation and size.
        value: ValueSummaryReport,
    },
    /// An operation completed successfully.
    StepCompleted {
        /// Output representation and size.
        output: ValueSummaryReport,
    },
    /// Execution paused before invoking an operation.
    BreakpointReached {
        /// Unconsumed input representation and size.
        input: ValueSummaryReport,
    },
    /// Execution failed without retaining an error payload.
    ExecutionFailed {
        /// Stable machine-readable failure code.
        code: String,
    },
}

/// Representation and logical payload size recorded in a trace.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ValueSummaryReport {
    /// Portable value representation.
    pub kind: ValueKind,
    /// Saturating logical payload size in bytes.
    pub size_bytes: u64,
}

impl ExecutionReport {
    /// Builds a report from a successful executor result and retained artifact.
    #[must_use]
    pub fn from_execution(result: &ExecutionResult, artifact: &ArtifactMeta) -> Self {
        Self {
            schema: "ferrosift.execution.v1",
            status: ReportStatus::from(result.status),
            artifact_id: artifact.id.as_str().to_owned(),
            value_kind: result.value.kind(),
            value_size_bytes: artifact.size_bytes,
            digest_sha256: artifact.digest_sha256.clone(),
            value: result.value.clone(),
            runtime_version: env!("CARGO_PKG_VERSION"),
            trace: TraceEventReport::from_trace(&result.trace),
            warnings: Vec::new(),
        }
    }
}

impl InspectReport {
    /// Builds an inspect report with an optional bounded preview.
    #[must_use]
    pub fn new(
        meta: &ArtifactMeta,
        value: &Value,
        max_preview_bytes: usize,
        include_preview: bool,
    ) -> Self {
        let mut preview_hex = None;
        let mut preview_text = None;
        let mut preview_truncated = false;
        if include_preview {
            match value {
                Value::Bytes(bytes) => {
                    let (slice, truncated) = take_preview(bytes, max_preview_bytes);
                    preview_hex = Some(hex_encode(slice));
                    preview_truncated = truncated;
                }
                Value::Text(text)
                    if matches!(
                        text.encoding,
                        ferrosift_model::TextEncoding::Utf8
                    ) =>
                {
                    let bytes = text.text.as_bytes();
                    let (slice, truncated) = take_preview(bytes, max_preview_bytes);
                    preview_hex = Some(hex_encode(slice));
                    preview_text = Some(String::from_utf8_lossy(slice).into_owned());
                    preview_truncated = truncated;
                }
                _ => {}
            }
        }
        Self {
            schema: "ferrosift.inspect.v1",
            artifact_id: meta.id.as_str().to_owned(),
            value_kind: meta.kind,
            size_bytes: meta.size_bytes,
            digest_sha256: meta.digest_sha256.clone(),
            preview_hex,
            preview_truncated,
            preview_text,
        }
    }
}

impl From<ExecutionStatus> for ReportStatus {
    fn from(status: ExecutionStatus) -> Self {
        match status {
            ExecutionStatus::Completed => Self::Completed,
            ExecutionStatus::Paused { step_index } => Self::Paused { step_index },
        }
    }
}

impl TraceEventReport {
    fn from_trace(trace: &ExecutionTrace) -> Vec<Self> {
        trace.events.iter().map(Self::from).collect()
    }
}

impl From<&TraceEvent> for TraceEventReport {
    fn from(event: &TraceEvent) -> Self {
        Self {
            location: StepLocationReport::from(&event.location),
            kind: TraceEventKindReport::from(&event.kind),
        }
    }
}

impl From<&StepLocation> for StepLocationReport {
    fn from(location: &StepLocation) -> Self {
        Self {
            index: location.index,
            step_id: location.step_id.as_str().into(),
            operation: location.operation.as_str().into(),
        }
    }
}

impl From<&TraceEventKind> for TraceEventKindReport {
    fn from(kind: &TraceEventKind) -> Self {
        match kind {
            TraceEventKind::StepStarted { input } => Self::StepStarted {
                input: ValueSummaryReport::from(*input),
            },
            TraceEventKind::StepSkipped { value } => Self::StepSkipped {
                value: ValueSummaryReport::from(*value),
            },
            TraceEventKind::StepCompleted { output } => Self::StepCompleted {
                output: ValueSummaryReport::from(*output),
            },
            TraceEventKind::BreakpointReached { input } => Self::BreakpointReached {
                input: ValueSummaryReport::from(*input),
            },
            TraceEventKind::ExecutionFailed { code } => Self::ExecutionFailed {
                code: code.clone(),
            },
        }
    }
}

impl From<ValueSummary> for ValueSummaryReport {
    fn from(summary: ValueSummary) -> Self {
        Self {
            kind: summary.kind,
            size_bytes: summary.size_bytes,
        }
    }
}

fn take_preview(bytes: &[u8], max_preview_bytes: usize) -> (&[u8], bool) {
    if bytes.len() > max_preview_bytes {
        (&bytes[..max_preview_bytes], true)
    } else {
        (bytes, false)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = core::fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}"));
    }
    out
}
