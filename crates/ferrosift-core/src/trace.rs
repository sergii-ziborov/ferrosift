//! Bounded execution traces that never retain operation values.

use alloc::{string::String, vec::Vec};

use ferrosift_model::{OperationId, StepId, Value, ValueKind};

use crate::value_size::logical_size;

/// Representation and logical payload size recorded in a trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueSummary {
    /// Portable value representation.
    pub kind: ValueKind,
    /// Saturating logical payload size in bytes.
    pub size_bytes: u64,
}

impl ValueSummary {
    /// Creates a bounded summary without retaining the supplied value.
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        Self {
            kind: value.kind(),
            size_bytes: logical_size(value),
        }
    }
}

/// Stable location of one recipe step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepLocation {
    /// Zero-based recipe position.
    pub index: usize,
    /// Stable recipe-local step identity.
    pub step_id: StepId,
    /// Canonical operation identity.
    pub operation: OperationId,
}

/// Bounded information emitted for one execution transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceEventKind {
    /// An enabled operation is about to run.
    StepStarted {
        /// Input representation and size.
        input: ValueSummary,
    },
    /// A disabled operation preserved its input.
    StepSkipped {
        /// Preserved value representation and size.
        value: ValueSummary,
    },
    /// An operation completed successfully.
    StepCompleted {
        /// Output representation and size.
        output: ValueSummary,
    },
    /// Execution paused before invoking an operation.
    BreakpointReached {
        /// Unconsumed input representation and size.
        input: ValueSummary,
    },
    /// Execution failed without retaining an error payload.
    ExecutionFailed {
        /// Stable machine-readable failure code.
        code: String,
    },
}

/// One ordered trace event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    /// Recipe step associated with the transition.
    pub location: StepLocation,
    /// Bounded transition details.
    pub kind: TraceEventKind,
}

/// Ordered bounded trace for one executor invocation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionTrace {
    /// Events in execution order.
    pub events: Vec<TraceEvent>,
}

/// Terminal state of a successful executor invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionStatus {
    /// Every executable step completed.
    Completed,
    /// Execution paused before the indicated step.
    Paused {
        /// Zero-based position of the unexecuted step.
        step_index: usize,
    },
}

/// Value, status, and bounded trace returned after successful execution or pause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    /// Whether the recipe completed or paused at a breakpoint.
    pub status: ExecutionStatus,
    /// Current value after the last completed or skipped step.
    pub value: Value,
    /// Bounded ordered execution trace.
    pub trace: ExecutionTrace,
}
