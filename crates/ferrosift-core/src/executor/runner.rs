//! Recursive region interpreter for recipes, including nested Fork/Merge.

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
    Pause { step_index: usize },
}

struct Runner<'a> {
    value: Value,
    trace: ExecutionTrace,
    budget: ExecutionBudget,
    initial_input_size: u64,
    cancellation: &'a dyn Cancellation,
    capabilities: CapabilitySet,
    /// Current nested Fork depth (0 at the top-level recipe).
    flow_depth: usize,
    /// Total operation invocations across the whole run (including every branch).
    operation_invocations: u64,
    /// Total bytes processed (branch inputs + operation inputs).
    total_bytes_processed: u64,
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
    /// Interpret steps in half-open range `[start, end)` against `self.value`.
    ///
    /// This is the single recursive control-flow interpreter: normal ops,
    /// Fork regions, and future conditionals/subsections should all go through
    /// here so nested regions compose.
    fn execute_region(
        &mut self,
        start: usize,
        end: usize,
        prepared: &[PreparedStep<'_, '_>],
    ) -> Result<StepControl, ExecutionError> {
        let mut index = start;
        while index < end {
            if !prepared[index].step.disabled
                && flow::is_fork(&prepared[index].step.operation)
                && prepared[index].operation.is_some()
            {
                let merge_index = find_merge(index, prepared)
                    .unwrap_or(end)
                    .min(end);
                if let StepControl::Pause { step_index } =
                    self.run_fork(index, merge_index, end, prepared)?
                {
                    return Ok(StepControl::Pause { step_index });
                }
                index = if merge_index < end
                    && !prepared[merge_index].step.disabled
                    && flow::is_merge(&prepared[merge_index].step.operation)
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

    fn run_fork(
        &mut self,
        fork_index: usize,
        merge_index: usize,
        region_end: usize,
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
            return Ok(StepControl::Pause {
                step_index: fork_index,
            });
        }

        self.enter_flow(&location)?;
        self.count_invocation(&location)?;
        let input_summary = ValueSummary::from_value(&self.value);
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted {
                input: input_summary,
            },
        });

        let (split, merge, ignore_errors) = fork_delimiters(&fork.arguments);
        let branches = self.split_branches(&split, &location)?;
        let mapped = self.map_branches(
            branches,
            fork_index + 1,
            merge_index.min(region_end),
            ignore_errors,
            prepared,
            &location,
        )?;
        let outputs = match mapped {
            BranchMap::Done(outputs) => outputs,
            BranchMap::Pause { step_index } => {
                self.leave_flow();
                return Ok(StepControl::Pause { step_index });
            }
        };
        self.leave_flow();

        let output = Value::Text(TextValue {
            text: outputs.join(merge.as_str()),
            encoding: TextEncoding::Utf8,
        });
        self.finish_fork(
            output,
            input_summary.size_bytes,
            merge_index,
            region_end,
            prepared,
            location,
        )
    }

    fn split_branches(
        &mut self,
        split: &str,
        location: &StepLocation,
    ) -> Result<Vec<String>, ExecutionError> {
        let input_text = value_as_text(mem::replace(&mut self.value, Value::Empty))
            .map_err(|failure| self.fail(failure, location.clone()))?;
        let branches: Vec<String> = if input_text.is_empty() {
            Vec::new()
        } else {
            input_text.split(split).map(String::from).collect()
        };
        if branches.len() > self.budget.max_branches {
            return Err(self.fail(ExecutionFailure::BranchLimitExceeded, location.clone()));
        }
        Ok(branches)
    }

    fn map_branches(
        &mut self,
        branches: Vec<String>,
        body_start: usize,
        body_end: usize,
        ignore_errors: bool,
        prepared: &[PreparedStep<'_, '_>],
        location: &StepLocation,
    ) -> Result<BranchMap, ExecutionError> {
        let mut outputs = Vec::with_capacity(branches.len());
        for branch in branches {
            if self.cancellation.is_cancelled() {
                return Err(self.fail(
                    ExecutionFailure::Operation(OperationError::Cancelled),
                    location.clone(),
                ));
            }
            self.account_bytes(branch.len() as u64, location)?;
            self.value = Value::Text(TextValue {
                text: branch,
                encoding: TextEncoding::Utf8,
            });
            match self.execute_region(body_start, body_end, prepared) {
                Ok(StepControl::Continue) => {
                    match value_as_text(mem::replace(&mut self.value, Value::Empty)) {
                        Ok(text) => {
                            self.account_bytes(text.len() as u64, location)?;
                            outputs.push(text);
                        }
                        Err(_) if ignore_errors => outputs.push(String::new()),
                        Err(failure) => return Err(self.fail(failure, location.clone())),
                    }
                }
                Ok(StepControl::Pause { step_index }) => {
                    return Ok(BranchMap::Pause { step_index });
                }
                Err(_) if ignore_errors => {
                    self.value = Value::Empty;
                    outputs.push(String::new());
                }
                Err(error) => return Err(error),
            }
        }
        Ok(BranchMap::Done(outputs))
    }

    fn finish_fork(
        &mut self,
        output: Value,
        input_size: u64,
        merge_index: usize,
        region_end: usize,
        prepared: &[PreparedStep<'_, '_>],
        location: StepLocation,
    ) -> Result<StepControl, ExecutionError> {
        let output_summary = ValueSummary::from_value(&output);
        if let Err(failure) = limits::check_output(
            output_summary.size_bytes,
            input_size,
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
        self.emit_merge_events(merge_index, region_end, prepared, &output)?;
        self.value = output;
        Ok(StepControl::Continue)
    }

    fn emit_merge_events(
        &mut self,
        merge_index: usize,
        region_end: usize,
        prepared: &[PreparedStep<'_, '_>],
        output: &Value,
    ) -> Result<(), ExecutionError> {
        if merge_index < region_end
            && merge_index < prepared.len()
            && !prepared[merge_index].step.disabled
            && flow::is_merge(&prepared[merge_index].step.operation)
        {
            let merge_location = step_location(merge_index, prepared[merge_index].step);
            self.count_invocation(&merge_location)?;
            let summary = ValueSummary::from_value(output);
            self.trace.events.push(TraceEvent {
                location: merge_location.clone(),
                kind: TraceEventKind::StepStarted { input: summary },
            });
            self.trace.events.push(TraceEvent {
                location: merge_location,
                kind: TraceEventKind::StepCompleted { output: summary },
            });
        }
        Ok(())
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
            return Ok(StepControl::Pause {
                step_index: index,
            });
        }

        // Stray Merge is identity (join already happened in run_fork).
        if flow::is_merge(&prepared.step.operation) {
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
        if !operation.spec().input.accepts(input_summary.kind) {
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

    fn enter_flow(&mut self, location: &StepLocation) -> Result<(), ExecutionError> {
        self.flow_depth = self.flow_depth.saturating_add(1);
        if self.flow_depth > self.budget.max_flow_depth {
            return Err(self.fail(ExecutionFailure::FlowDepthExceeded, location.clone()));
        }
        Ok(())
    }

    fn leave_flow(&mut self) {
        self.flow_depth = self.flow_depth.saturating_sub(1);
    }

    fn count_invocation(&mut self, location: &StepLocation) -> Result<(), ExecutionError> {
        self.operation_invocations = self.operation_invocations.saturating_add(1);
        if self.operation_invocations > self.budget.max_operation_invocations {
            return Err(self.fail(ExecutionFailure::InvocationLimitExceeded, location.clone()));
        }
        Ok(())
    }

    fn account_bytes(
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

    fn fail(&mut self, failure: ExecutionFailure, location: StepLocation) -> ExecutionError {
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

enum BranchMap {
    Done(Vec<String>),
    Pause { step_index: usize },
}

fn fork_delimiters(arguments: &ferrosift_model::Arguments) -> (String, String, bool) {
    (
        flow::parse_delimiter(&text_arg(arguments, "split_delimiter")),
        flow::parse_delimiter(&text_arg(arguments, "merge_delimiter")),
        bool_arg(arguments, "ignore_errors"),
    )
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
