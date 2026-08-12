//! Structured executor failures with exact locations and partial traces.

use alloc::string::String;
use core::{error::Error, fmt};

use ferrosift_model::{HostCapability, ModelError};

use crate::{ExecutionTrace, OperationError, StepLocation, TraceEvent, TraceEventKind};

/// Typed reason an executor invocation failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionFailure {
    /// The recipe violates portable model invariants.
    InvalidRecipe(ModelError),
    /// The recipe contains more steps than the configured ceiling.
    StepLimitExceeded,
    /// The initial input exceeds the configured ceiling.
    InputLimitExceeded,
    /// An enabled step references an unregistered canonical operation.
    UnknownOperation,
    /// An operation requires a host capability that was not granted.
    CapabilityDenied {
        /// First missing capability in deterministic order.
        capability: HostCapability,
    },
    /// A step received a value representation outside its declared contract.
    InputKindMismatch,
    /// A step produced a value representation outside its declared contract.
    OutputKindMismatch,
    /// A produced value exceeds the configured output ceiling.
    OutputLimitExceeded,
    /// A produced value exceeds the configured per-step or total expansion ratio.
    ExpansionRatioExceeded,
    /// A Fork produced more branches than the configured ceiling.
    BranchLimitExceeded,
    /// Nested flow depth exceeded the configured ceiling.
    FlowDepthExceeded,
    /// Total operation invocations exceeded the configured ceiling.
    InvocationLimitExceeded,
    /// Total bytes processed across branches/ops exceeded the configured ceiling.
    WorkLimitExceeded,
    /// Validation or execution failed through the portable operation boundary.
    Operation(OperationError),
}

impl ExecutionFailure {
    fn code(&self) -> &str {
        match self {
            Self::InvalidRecipe(error) => error.code(),
            Self::StepLimitExceeded => "core.executor.step_limit_exceeded",
            Self::InputLimitExceeded => "core.executor.input_limit_exceeded",
            Self::UnknownOperation => "core.executor.operation_unknown",
            Self::CapabilityDenied { .. } => "core.executor.capability_denied",
            Self::InputKindMismatch => "core.executor.input_kind_mismatch",
            Self::OutputKindMismatch => "core.executor.output_kind_mismatch",
            Self::OutputLimitExceeded => "core.executor.output_limit_exceeded",
            Self::ExpansionRatioExceeded => "core.executor.expansion_ratio_exceeded",
            Self::BranchLimitExceeded => "core.executor.branch_limit_exceeded",
            Self::FlowDepthExceeded => "core.executor.flow_depth_exceeded",
            Self::InvocationLimitExceeded => "core.executor.invocation_limit_exceeded",
            Self::WorkLimitExceeded => "core.executor.work_limit_exceeded",
            Self::Operation(error) => error.code(),
        }
    }
}

/// Executor failure with an optional exact step location and bounded partial trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionError {
    /// Typed failure reason.
    pub failure: ExecutionFailure,
    /// Responsible step, or `None` for recipe-wide preflight failures.
    pub location: Option<StepLocation>,
    /// Bounded events emitted before and including the failure.
    pub trace: ExecutionTrace,
}

impl ExecutionError {
    /// Returns the stable machine-readable failure code.
    #[must_use]
    pub fn code(&self) -> &str {
        self.failure.code()
    }

    pub(super) fn global(failure: ExecutionFailure) -> Self {
        Self {
            failure,
            location: None,
            trace: ExecutionTrace::default(),
        }
    }

    pub(super) fn at_step(
        failure: ExecutionFailure,
        location: StepLocation,
        mut trace: ExecutionTrace,
    ) -> Self {
        trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::ExecutionFailed {
                code: String::from(failure.code()),
            },
        });
        Self {
            failure,
            location: Some(location),
            trace,
        }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.failure {
            ExecutionFailure::InvalidRecipe(error) => Some(error),
            ExecutionFailure::Operation(error) => Some(error),
            _ => None,
        }
    }
}
