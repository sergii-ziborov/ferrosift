//! Recursive region interpreter for recipes, including nested Fork/Merge.

use alloc::vec::Vec;
use core::mem;

use ferrosift_model::{ArgumentValue, CapabilitySet, Value, ValueConstraint, ValueKind};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, ExecutionStatus, ExecutionTrace,
    OperationContext, OperationError, StepLocation, TraceEvent, TraceEventKind, ValueSummary,
};

use super::{ExecutionError, ExecutionFailure, flow, limits, preflight::PreparedStep};

pub(super) enum StepControl {
    Continue,
    Pause { step_index: usize },
}

pub(super) struct Runner<'a> {
    pub(super) value: Value,
    pub(super) trace: ExecutionTrace,
    pub(super) budget: ExecutionBudget,
    pub(super) initial_input_size: u64,
    pub(super) cancellation: &'a dyn Cancellation,
    pub(super) capabilities: CapabilitySet,
    /// Current nested Fork depth (0 at the top-level recipe).
    pub(super) flow_depth: usize,
    /// Total operation invocations across the whole run (including every branch).
    pub(super) operation_invocations: u64,
    /// Total bytes processed (branch inputs + operation inputs).
    pub(super) total_bytes_processed: u64,
}

pub(super) fn run(
    prepared: &[PreparedStep<'_>],
    input: Value,
    budget: ExecutionBudget,
    initial_input_size: u64,
    cancellation: &dyn Cancellation,
    capabilities: CapabilitySet,
) -> Result<ExecutionResult, ExecutionError> {
    let mut runner = Runner {
        value: input,
        trace: ExecutionTrace::default(),
        budget,
        initial_input_size,
        cancellation,
        capabilities,
        flow_depth: 0,
        operation_invocations: 0,
        total_bytes_processed: initial_input_size,
    };
    match runner.execute_region(0, prepared.len(), prepared)? {
        StepControl::Pause { step_index } => {
            Ok(runner.finish(ExecutionStatus::Paused { step_index }))
        }
        StepControl::Continue => Ok(runner.finish(ExecutionStatus::Completed)),
    }
}

fn find_merge(fork_index: usize, prepared: &[PreparedStep<'_>]) -> Option<usize> {
    let ids: Vec<_> = prepared
        .iter()
        .map(|step| step.operation_id.clone())
        .collect();
    let merge_all: Vec<bool> = prepared
        .iter()
        .map(|step| match step.arguments.get("merge_all") {
            Some(ArgumentValue::Boolean(value)) => *value,
            _ => true,
        })
        .collect();
    let disabled: Vec<bool> = prepared.iter().map(|step| step.disabled).collect();
    flow::find_merge_index(fork_index, &ids, &merge_all, &disabled)
}

impl Runner<'_> {
    /// Interpret steps in half-open range `[start, end)` against `self.value`.
    ///
    /// This is the single recursive control-flow interpreter: normal ops,
    /// Fork regions, and future conditionals/subsections should all go through
    /// here so nested regions compose.
    pub(super) fn execute_region(
        &mut self,
        start: usize,
        end: usize,
        prepared: &[PreparedStep<'_>],
    ) -> Result<StepControl, ExecutionError> {
        let mut index = start;
        while index < end {
            if !prepared[index].disabled
                && flow::is_fork(&prepared[index].operation_id)
                && prepared[index].operation.is_some()
            {
                let merge_index = find_merge(index, prepared).unwrap_or(end).min(end);
                if let StepControl::Pause { step_index } =
                    self.run_fork(index, merge_index, end, prepared)?
                {
                    return Ok(StepControl::Pause { step_index });
                }
                index = if merge_index < end
                    && !prepared[merge_index].disabled
                    && flow::is_merge(&prepared[merge_index].operation_id)
                {
                    merge_index + 1
                } else {
                    merge_index
                };
                continue;
            }

            match self.run_step(index, &prepared[index])? {
                StepControl::Pause { step_index } => {
                    return Ok(StepControl::Pause { step_index });
                }
                StepControl::Continue => index += 1,
            }
        }
        Ok(StepControl::Continue)
    }

    fn run_step(
        &mut self,
        index: usize,
        prepared: &PreparedStep<'_>,
    ) -> Result<StepControl, ExecutionError> {
        let location = prepared.location(index);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        if prepared.disabled {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::StepSkipped {
                    value: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepControl::Continue);
        }
        if prepared.breakpoint {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::BreakpointReached {
                    input: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepControl::Pause { step_index: index });
        }

        // Stray Merge is identity (join already happened in run_fork).
        if flow::is_merge(&prepared.operation_id) {
            self.count_invocation(&location)?;
            let summary = ValueSummary::from_value(&self.value);
            self.trace.events.push(TraceEvent {
                location: location.clone(),
                kind: TraceEventKind::StepStarted { input: summary },
            });
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::StepCompleted { output: summary },
            });
            return Ok(StepControl::Continue);
        }

        // Nested Fork is never handled here: execute_region intercepts it.
        // Standalone Fork (empty body) is still an operation.execute path only
        // when not detected as a region — but execute_region always detects it.

        let Some(operation) = prepared.operation else {
            return Err(self.fail(ExecutionFailure::UnknownOperation, location));
        };
        let input_summary = ValueSummary::from_value(&self.value);
        // Accepted outright, or convertible into something accepted. The
        // second half is what lets markup reach an operation that wants text:
        // the reference converts there rather than refusing, and refusing
        // would reject a recipe that runs against it.
        let accepted = ValueKind::ALL.iter().copied().any(|target| {
            operation.spec().input.accepts(target) && input_summary.kind.converts_to(target)
        });
        if !accepted {
            return Err(self.fail(ExecutionFailure::InputKindMismatch, location));
        }
        self.count_invocation(&location)?;
        self.account_bytes(input_summary.size_bytes, &location)?;
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted {
                input: input_summary,
            },
        });
        let mut context =
            OperationContext::new(self.budget, self.cancellation, self.capabilities.clone());
        let input = mem::replace(&mut self.value, Value::Empty);
        // Re-read the value as the step expects it before handing it over.
        // The reference does this whenever a dish is asked for another type,
        // and two of those conversions lose information -- markup arrives with
        // its tags removed, a number as the digits JavaScript prints. Passing
        // the value through untouched would run a different recipe than the
        // same steps run there. Preflight has already agreed a conversion
        // exists, so a value that cannot convert is left as it is and the
        // operation reports the mismatch itself.
        let input = adapt(input, &operation.spec().input);
        let output = match operation.execute(input, &prepared.arguments, &mut context) {
            Ok(output) => output,
            Err(error) => {
                return Err(self.fail(ExecutionFailure::Operation(error), location));
            }
        };
        let output_summary = ValueSummary::from_value(&output);
        if !operation.spec().output.accepts(output_summary.kind) {
            return Err(self.fail(ExecutionFailure::OutputKindMismatch, location));
        }
        if let Err(failure) = limits::check_output(
            output_summary.size_bytes,
            input_summary.size_bytes,
            self.initial_input_size,
            self.budget,
            operation.spec().output_behavior,
        ) {
            return Err(self.fail(failure, location));
        }
        self.account_bytes(output_summary.size_bytes, &location)?;
        self.trace.events.push(TraceEvent {
            location,
            kind: TraceEventKind::StepCompleted {
                output: output_summary,
            },
        });
        self.value = output;
        Ok(StepControl::Continue)
    }

    pub(super) fn enter_flow(&mut self, location: &StepLocation) -> Result<(), ExecutionError> {
        self.flow_depth = self.flow_depth.saturating_add(1);
        if self.flow_depth > self.budget.max_flow_depth {
            return Err(self.fail(ExecutionFailure::FlowDepthExceeded, location.clone()));
        }
        Ok(())
    }

    pub(super) fn leave_flow(&mut self) {
        self.flow_depth = self.flow_depth.saturating_sub(1);
    }

    pub(super) fn count_invocation(
        &mut self,
        location: &StepLocation,
    ) -> Result<(), ExecutionError> {
        self.operation_invocations = self.operation_invocations.saturating_add(1);
        if self.operation_invocations > self.budget.max_operation_invocations {
            return Err(self.fail(ExecutionFailure::InvocationLimitExceeded, location.clone()));
        }
        Ok(())
    }

    pub(super) fn account_bytes(
        &mut self,
        size: u64,
        location: &StepLocation,
    ) -> Result<(), ExecutionError> {
        self.total_bytes_processed = self.total_bytes_processed.saturating_add(size);
        if self.total_bytes_processed > self.budget.max_total_bytes_processed {
            return Err(self.fail(ExecutionFailure::WorkLimitExceeded, location.clone()));
        }
        Ok(())
    }

    pub(super) fn fail(
        &mut self,
        failure: ExecutionFailure,
        location: StepLocation,
    ) -> ExecutionError {
        // Clone so soft-fail paths (Fork ignore_errors) can drop the error
        // without wiping the runner's continuing trace.
        ExecutionError::at_step(failure, location, self.trace.clone())
    }

    fn finish(self, status: ExecutionStatus) -> ExecutionResult {
        ExecutionResult {
            status,
            value: self.value,
            trace: self.trace,
        }
    }
}

/// Re-reads a value as the constraint asks for, when the model defines how.
///
/// Left untouched when the constraint already accepts what the value is, or
/// when no conversion exists -- the operation is then the one that reports the
/// mismatch, with the kind it actually received.
fn adapt(value: Value, constraint: &ValueConstraint) -> Value {
    if constraint.accepts(value.kind()) {
        return value;
    }
    let kind = value.kind();
    for target in ValueKind::ALL {
        if !constraint.accepts(target) || !kind.converts_to(target) {
            continue;
        }
        if let Some(converted) = value.clone().reinterpret(target) {
            return converted;
        }
    }
    value
}