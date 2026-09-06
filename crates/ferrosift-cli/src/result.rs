//! Structured execution reports shared by CLI transports.

use ferrosift_core::{
    ExecutionResult, ExecutionStatus, ExecutionTrace, StepLocation, TraceEvent, TraceEventKind,
    ValueSummary,
};
use ferrosift_model::{Value, ValueKind};
use serde::Serialize;

/// Machine-readable envelope for one recipe execution.
#[derive(Debug, Serialize)]
pub struct ExecutionReport<'a> {
    /// Stable schema identifier for this envelope.
    pub schema: &'static str,
    /// Whether the recipe completed or paused.
    pub status: ReportStatus,
    /// Representation of the current value.
    pub value_kind: ValueKind,
    /// Saturating logical payload size of the current value.
    pub value_size_bytes: u64,
    /// Current value after the last completed or skipped step.
    pub value: &'a Value,
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

impl<'a> ExecutionReport<'a> {
    /// Builds a report from a successful executor result.
    #[must_use]
    pub fn from_execution(result: &'a ExecutionResult) -> Self {
        Self {
            schema: "ferrosift.execution.v1",
            status: ReportStatus::from(result.status),
            value_kind: result.value.kind(),
            value_size_bytes: ValueSummary::from_value(&result.value).size_bytes,
            value: &result.value,
            runtime_version: env!("CARGO_PKG_VERSION"),
            trace: TraceEventReport::from_trace(&result.trace),
            warnings: Vec::new(),
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
