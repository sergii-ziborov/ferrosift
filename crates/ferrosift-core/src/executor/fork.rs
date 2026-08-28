//! Fork/Merge branch mapping for the region interpreter.

use alloc::{string::String, vec::Vec};
use core::mem;

use ferrosift_model::{ArgumentValue, OutputBehavior, TextEncoding, TextValue, Value};

use crate::{OperationError, StepLocation, TraceEvent, TraceEventKind, ValueSummary};

use super::preflight::PreparedStep;
use super::runner::{Runner, StepControl};
use super::{ExecutionError, ExecutionFailure, flow, limits};

impl Runner<'_> {
    pub(super) fn run_fork(
        &mut self,
        fork_index: usize,
        merge_index: usize,
        region_end: usize,
        prepared: &[PreparedStep<'_>],
    ) -> Result<StepControl, ExecutionError> {
        let fork = &prepared[fork_index];
        let location = fork.location(fork_index);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        if fork.breakpoint {
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
        prepared: &[PreparedStep<'_>],
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
            let outcome = self
                .in_nested_recipe(|runner| runner.execute_region(body_start, body_end, prepared));
            match outcome {
                // A `Return` inside a branch ends that branch, not the run: the
                // reference gives each branch its own recipe to return from.
                Ok(StepControl::Continue | StepControl::Stop) => {
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
        prepared: &[PreparedStep<'_>],
        location: StepLocation,
    ) -> Result<StepControl, ExecutionError> {
        let output_summary = ValueSummary::from_value(&output);
        // A merge is not an operation and has no spec to consult. Its output is
        // the branches joined, so it is proportional by construction and keeps
        // the ratio check — a fork that multiplied its input is exactly the
        // growth that check exists to catch.
        if let Err(failure) = limits::check_output(
            output_summary.size_bytes,
            input_size,
            self.initial_input_size,
            self.budget,
            OutputBehavior::InputProportional,
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

    pub(super) fn emit_merge_events(
        &mut self,
        merge_index: usize,
        region_end: usize,
        prepared: &[PreparedStep<'_>],
        output: &Value,
    ) -> Result<(), ExecutionError> {
        if merge_index < region_end
            && merge_index < prepared.len()
            && !prepared[merge_index].disabled
            && flow::is_merge(&prepared[merge_index].operation_id)
        {
            let merge_location = prepared[merge_index].location(merge_index);
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

/// The reference's own string view of a dish: text as it stands, and bytes as
/// one character per byte when they are not valid UTF-8.
pub(super) fn value_as_text(value: Value) -> Result<String, ExecutionFailure> {
    match value {
        Value::Text(text) => Ok(text.text),
        Value::Bytes(bytes) => Ok(match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => error.into_bytes().into_iter().map(char::from).collect(),
        }),
        _ => Err(ExecutionFailure::Operation(
            OperationError::InvalidArguments,
        )),
    }
}
