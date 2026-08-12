//! Runtime state machine for prepared recipes, including Fork/Merge map regions.

use alloc::{string::String, vec::Vec};
use core::mem;

use ferrosift_model::{ArgumentValue, CapabilitySet, TextEncoding, TextValue, Value};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, ExecutionStatus, ExecutionTrace,
    OperationContext, OperationError, StepLocation, TraceEvent, TraceEventKind, ValueSummary,
};

use super::{
    ExecutionError, ExecutionFailure, flow, limits, preflight::PreparedStep, step_location,
};

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
    let mut index = 0;
    while index < prepared.len() {
        if !prepared[index].step.disabled
            && flow::is_fork(&prepared[index].step.operation)
            && prepared[index].operation.is_some()
        {
            let merge_index = find_merge(index, prepared).unwrap_or(prepared.len());
            if matches!(
                runner.run_fork(index, merge_index, prepared)?,
                StepControl::Pause
            ) {
                return Ok(runner.finish(ExecutionStatus::Paused { step_index: index }));
            }
            index = if merge_index < prepared.len() {
                merge_index + 1
            } else {
                merge_index
            };
            continue;
        }
        if matches!(runner.run_step(index, &prepared[index])?, StepControl::Pause) {
            return Ok(runner.finish(ExecutionStatus::Paused { step_index: index }));
        }
        index += 1;
    }
    Ok(runner.finish(ExecutionStatus::Completed))
}

fn find_merge(fork_index: usize, prepared: &[PreparedStep<'_, '_>]) -> Option<usize> {
    let ids: Vec<_> = prepared
        .iter()
        .map(|step| step.step.operation.clone())
        .collect();
    let merge_all: Vec<bool> = prepared
        .iter()
        .map(|step| match step.arguments.get("merge_all") {
            Some(ArgumentValue::Boolean(value)) => *value,
            _ => true,
        })
        .collect();
    let disabled: Vec<bool> = prepared.iter().map(|step| step.step.disabled).collect();
    flow::find_merge_index(fork_index, &ids, &merge_all, &disabled)
}

impl Runner<'_> {
    fn run_fork(
        &mut self,
        fork_index: usize,
        merge_index: usize,
        prepared: &[PreparedStep<'_, '_>],
    ) -> Result<StepControl, ExecutionError> {
        let fork = &prepared[fork_index];
        let location = step_location(fork_index, fork.step);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        if fork.step.breakpoint {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::BreakpointReached {
                    input: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepControl::Pause);
        }

        let input_summary = ValueSummary::from_value(&self.value);
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted {
                input: input_summary,
            },
        });

        let split = flow::parse_delimiter(&text_arg(&fork.arguments, "split_delimiter"));
        let merge = flow::parse_delimiter(&text_arg(&fork.arguments, "merge_delimiter"));
        let ignore_errors = bool_arg(&fork.arguments, "ignore_errors");

        let input_text = value_as_text(mem::replace(&mut self.value, Value::Empty)).map_err(
            |failure| self.fail(failure, location.clone()),
        )?;
        let branches: Vec<String> = if input_text.is_empty() {
            Vec::new()
        } else {
            input_text
                .split(split.as_str())
                .map(String::from)
                .collect()
        };

        let body = &prepared[fork_index + 1..merge_index];
        let mut outputs = Vec::with_capacity(branches.len());
        for branch in branches {
            if self.cancellation.is_cancelled() {
                return Err(self.fail(
                    ExecutionFailure::Operation(OperationError::Cancelled),
                    location,
                ));
            }
            match self.run_branch(fork_index + 1, body, branch) {
                Ok(text) => outputs.push(text),
                Err(error) if ignore_errors => {
                    let _ = error;
                    outputs.push(String::new());
                }
                Err(error) => return Err(error),
            }
        }

        let joined = outputs.join(merge.as_str());
        let output = Value::Text(TextValue {
            text: joined,
            encoding: TextEncoding::Utf8,
        });
        let output_summary = ValueSummary::from_value(&output);
        if let Err(failure) = limits::check_output(
            output_summary.size_bytes,
            input_summary.size_bytes,
            self.initial_input_size,
            self.budget,
        ) {
            return Err(self.fail(failure, location));
        }
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepCompleted {
                output: output_summary,
            },
        });

        // Record Merge as a completed identity at the join point when present.
        if merge_index < prepared.len() {
            let merge_step = &prepared[merge_index];
            if !merge_step.step.disabled {
                let merge_location = step_location(merge_index, merge_step.step);
                self.trace.events.push(TraceEvent {
                    location: merge_location.clone(),
                    kind: TraceEventKind::StepStarted {
                        input: ValueSummary::from_value(&output),
                    },
                });
                self.trace.events.push(TraceEvent {
                    location: merge_location,
                    kind: TraceEventKind::StepCompleted {
                        output: ValueSummary::from_value(&output),
                    },
                });
            }
        }

        self.value = output;
        Ok(StepControl::Continue)
    }

    fn run_branch(
        &mut self,
        body_start: usize,
        body: &[PreparedStep<'_, '_>],
        branch: String,
    ) -> Result<String, ExecutionError> {
        let mut value = Value::Text(TextValue {
            text: branch,
            encoding: TextEncoding::Utf8,
        });
        for (offset, prepared) in body.iter().enumerate() {
            let absolute = body_start + offset;
            if prepared.step.disabled {
                let location = step_location(absolute, prepared.step);
                self.trace.events.push(TraceEvent {
                    location,
                    kind: TraceEventKind::StepSkipped {
                        value: ValueSummary::from_value(&value),
                    },
                });
                continue;
            }
            value = self.invoke_step(absolute, prepared, value)?;
        }
        match value_as_text(value) {
            Ok(text) => Ok(text),
            Err(failure) => {
                let location = body.first().map_or_else(
                    || StepLocation {
                        index: body_start,
                        step_id: ferrosift_model::StepId::new("fork-branch")
                            .expect("valid step id"),
                        operation: ferrosift_model::OperationId::from_static(flow::FORK_ID),
                    },
                    |step| step_location(body_start, step.step),
                );
                Err(self.fail(failure, location))
            }
        }
    }

    fn invoke_step(
        &mut self,
        index: usize,
        prepared: &PreparedStep<'_, '_>,
        input: Value,
    ) -> Result<Value, ExecutionError> {
        let location = step_location(index, prepared.step);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        let Some(operation) = prepared.operation else {
            return Err(self.fail(ExecutionFailure::UnknownOperation, location));
        };
        let input_summary = ValueSummary::from_value(&input);
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
        Ok(output)
    }

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

fn text_arg(arguments: &ferrosift_model::Arguments, name: &str) -> String {
    match arguments.get(name) {
        Some(ArgumentValue::Text(value)) => value.clone(),
        _ => String::from("\\n"),
    }
}

fn bool_arg(arguments: &ferrosift_model::Arguments, name: &str) -> bool {
    match arguments.get(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        _ => false,
    }
}

fn value_as_text(value: Value) -> Result<String, ExecutionFailure> {
    match value {
        Value::Text(text) => Ok(text.text),
        Value::Bytes(bytes) => Ok(match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => error.into_bytes().into_iter().map(char::from).collect(),
        }),
        _ => Err(ExecutionFailure::Operation(OperationError::InvalidArguments)),
    }
}
