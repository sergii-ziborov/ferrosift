//! Runtime state machine for already-prepared linear recipes.

use core::mem;

use ferrosift_model::{CapabilitySet, Value};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, ExecutionStatus, ExecutionTrace,
    OperationContext, OperationError, StepLocation, TraceEvent, TraceEventKind, ValueSummary,
};

use super::{ExecutionError, ExecutionFailure, limits, preflight::PreparedStep, step_location};

enum StepControl {
    Continue,
    Pause,
}

struct Runner<'a> {
    value: Value,
    trace: ExecutionTrace,
    budget: ExecutionBudget,
    initial_input_size: u64,
    cancellation: &'a dyn Cancellation,
    capabilities: CapabilitySet,
}

pub(super) fn run(
    prepared: &[PreparedStep<'_, '_>],
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
    };
    for (index, step) in prepared.iter().enumerate() {
        if matches!(runner.run_step(index, step)?, StepControl::Pause) {
            return Ok(runner.finish(ExecutionStatus::Paused { step_index: index }));
        }
    }
    Ok(runner.finish(ExecutionStatus::Completed))
}

impl Runner<'_> {
    fn run_step(
        &mut self,
        index: usize,
        prepared: &PreparedStep<'_, '_>,
    ) -> Result<StepControl, ExecutionError> {
        let location = step_location(index, prepared.step);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        if prepared.step.disabled {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::StepSkipped {
                    value: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepControl::Continue);
        }
        if prepared.step.breakpoint {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::BreakpointReached {
                    input: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepControl::Pause);
        }

        let Some(operation) = prepared.operation else {
            return Err(self.fail(ExecutionFailure::UnknownOperation, location));
        };
        let input_summary = ValueSummary::from_value(&self.value);
        if !operation.spec().input.accepts(input_summary.kind) {
            return Err(self.fail(ExecutionFailure::InputKindMismatch, location));
        }
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted {
                input: input_summary,
            },
        });
        let mut context =
            OperationContext::new(self.budget, self.cancellation, self.capabilities.clone());
        let input = mem::replace(&mut self.value, Value::Empty);
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
        ) {
            return Err(self.fail(failure, location));
        }
        self.trace.events.push(TraceEvent {
            location,
            kind: TraceEventKind::StepCompleted {
                output: output_summary,
            },
        });
        self.value = output;
        Ok(StepControl::Continue)
    }

    fn fail(&mut self, failure: ExecutionFailure, location: StepLocation) -> ExecutionError {
        ExecutionError::at_step(failure, location, mem::take(&mut self.trace))
    }

    fn finish(self, status: ExecutionStatus) -> ExecutionResult {
        ExecutionResult {
            status,
            value: self.value,
            trace: self.trace,
        }
    }
}
