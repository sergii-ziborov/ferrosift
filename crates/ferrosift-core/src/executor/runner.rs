//! Recursive region interpreter for recipes, including nested Fork/Merge.

use alloc::vec::Vec;
use core::mem;

use ferrosift_model::{ArgumentValue, CapabilitySet, Value, ValueConstraint, ValueKind};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, ExecutionStatus, ExecutionTrace, FlowDirective,
    OperationContext, OperationError, StepLocation, TraceEvent, TraceEventKind, ValueSummary,
};

use super::{ExecutionError, ExecutionFailure, flow, limits, preflight::PreparedStep};

/// What a whole region reports to whoever asked it to run.
pub(super) enum StepControl {
    Continue,
    Pause {
        step_index: usize,
    },
    /// A `Return` ended this region early. The value in hand is the answer.
    Stop,
}

/// What one step reports to the region running it.
///
/// Separate from [`StepControl`] because a jump is resolved by the region that
/// owns the indices and never escapes it: a region returns `Continue`, `Pause`
/// or `Stop`, and `Jump` exists only between a step and its own loop.
pub(super) enum StepFlow {
    Continue,
    Pause {
        step_index: usize,
    },
    Stop,
    /// Resume this region at `target`.
    Jump {
        target: usize,
    },
}

pub(super) struct Runner<'a> {
    pub(super) value: Value,
    pub(super) trace: ExecutionTrace,
    pub(super) budget: ExecutionBudget,
    pub(super) initial_input_size: u64,
    pub(super) cancellation: &'a dyn Cancellation,
    pub(super) capabilities: CapabilitySet,
    /// Current nested Fork/Subsection depth (0 at the top-level recipe).
    pub(super) flow_depth: usize,
    /// Total operation invocations across the whole run (including every branch).
    pub(super) operation_invocations: u64,
    /// Total bytes processed (branch inputs + operation inputs).
    pub(super) total_bytes_processed: u64,
    /// Jumps taken so far, shared by every jump site as the reference shares it.
    ///
    /// One counter for the recipe rather than one per site: `CyberChef` keeps
    /// `numJumps` on the recipe's own execution state, so two jumps in the same
    /// loop spend the same allowance. A Fork branch and a Subsection tranche
    /// each run as their own recipe there, so each starts from zero — see
    /// [`Runner::in_nested_recipe`].
    pub(super) jumps_taken: u32,
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
        jumps_taken: 0,
    };
    match runner.execute_region(0, prepared.len(), prepared)? {
        StepControl::Pause { step_index } => {
            Ok(runner.finish(ExecutionStatus::Paused { step_index }))
        }
        // A recipe that ran off the end and one a `Return` stopped are the same
        // outcome: the value in hand, complete. The reference makes no
        // distinction either — `Return` moves the counter past the last step.
        StepControl::Continue | StepControl::Stop => Ok(runner.finish(ExecutionStatus::Completed)),
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

/// Where a region resumes once its Merge has been accounted for.
///
/// The Merge itself is not re-run: the region that closed already emitted its
/// events. When no real Merge closes the region — the recipe simply ended —
/// there is nothing to step past.
pub(super) fn after_merge(merge_index: usize, end: usize, prepared: &[PreparedStep<'_>]) -> usize {
    if merge_index < end
        && !prepared[merge_index].disabled
        && flow::is_merge(&prepared[merge_index].operation_id)
    {
        merge_index + 1
    } else {
        merge_index
    }
}

/// The first `Label` named `label` inside `[start, end)`.
///
/// Scoped to the region rather than to the whole recipe, which is also what the
/// reference does without saying so: a Fork body there is built into its own
/// `Recipe`, so `getLabelIndex` can only see the body's own labels. A jump out
/// of a branch is therefore not a jump at all — in both implementations.
///
/// A disabled Label is still a destination. The reference's search does not ask
/// whether the step is enabled, and landing on one changes nothing anyway,
/// since the step after it is where execution resumes.
fn find_label(
    label: &str,
    start: usize,
    end: usize,
    prepared: &[PreparedStep<'_>],
) -> Option<usize> {
    (start..end).find(|&index| {
        flow::is_label(&prepared[index].operation_id)
            && matches!(
                prepared[index].arguments.get("name"),
                Some(ArgumentValue::Text(name)) if name == label,
            )
    })
}

impl Runner<'_> {
    /// Interpret steps in half-open range `[start, end)` against `self.value`.
    ///
    /// This is the single recursive control-flow interpreter: ordinary
    /// operations, Fork and Subsection regions, and the jumps between them all
    /// go through here so nested regions compose.
    ///
    /// `index` is a program counter and not a cursor. A step may send it
    /// backwards, which is what makes a recipe able to loop and why every path
    /// through here counts an invocation.
    pub(super) fn execute_region(
        &mut self,
        start: usize,
        end: usize,
        prepared: &[PreparedStep<'_>],
    ) -> Result<StepControl, ExecutionError> {
        let mut index = start;
        while index < end {
            if !prepared[index].disabled
                && flow::opens_region(&prepared[index].operation_id)
                && prepared[index].operation.is_some()
            {
                let merge_index = find_merge(index, prepared).unwrap_or(end).min(end);
                let opened = if flow::is_fork(&prepared[index].operation_id) {
                    match self.run_fork(index, merge_index, end, prepared)? {
                        StepControl::Continue => StepFlow::Jump {
                            target: after_merge(merge_index, end, prepared),
                        },
                        StepControl::Pause { step_index } => StepFlow::Pause { step_index },
                        StepControl::Stop => StepFlow::Stop,
                    }
                } else {
                    self.run_subsection(index, merge_index, end, prepared)?
                };
                match opened {
                    StepFlow::Pause { step_index } => {
                        return Ok(StepControl::Pause { step_index });
                    }
                    StepFlow::Stop => return Ok(StepControl::Stop),
                    StepFlow::Jump { target } => index = target,
                    // Only a Subsection answers this: its pattern selected
                    // nothing to scope, so the following steps run on the whole
                    // value exactly as if it were not there.
                    StepFlow::Continue => index += 1,
                }
                continue;
            }

            match self.run_step(index, start, end, prepared)? {
                StepFlow::Pause { step_index } => {
                    return Ok(StepControl::Pause { step_index });
                }
                StepFlow::Stop => return Ok(StepControl::Stop),
                StepFlow::Jump { target } => index = target,
                StepFlow::Continue => index += 1,
            }
        }
        Ok(StepControl::Continue)
    }

    /// Runs a nested recipe with its own jump allowance, restoring the outer one.
    ///
    /// A Fork branch and a Subsection tranche are each a whole recipe in the
    /// reference, and `numJumps` is local to a recipe's execution there. Left
    /// shared, a loop inside one branch would spend the allowance of every
    /// branch after it.
    pub(super) fn in_nested_recipe<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        let outer = core::mem::take(&mut self.jumps_taken);
        let result = body(self);
        self.jumps_taken = outer;
        result
    }

    fn run_step(
        &mut self,
        index: usize,
        start: usize,
        end: usize,
        steps: &[PreparedStep<'_>],
    ) -> Result<StepFlow, ExecutionError> {
        let prepared = &steps[index];
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
            return Ok(StepFlow::Continue);
        }
        if prepared.breakpoint {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::BreakpointReached {
                    input: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepFlow::Pause { step_index: index });
        }

        // Stray Merge is identity (the join already happened in the region).
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
            return Ok(StepFlow::Continue);
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
            location: location.clone(),
            kind: TraceEventKind::StepCompleted {
                output: output_summary,
            },
        });
        self.value = output;
        // Asked of every step, answered by four. The value is already stored,
        // so a step that both transforms and directs -- none do today -- would
        // have its output kept whichever way the counter then moves.
        let directive = match operation.direct(&self.value, &prepared.arguments, &context) {
            Ok(directive) => directive,
            Err(error) => return Err(self.fail(ExecutionFailure::Operation(error), location)),
        };
        self.apply_directive(directive, start, end, steps, location)
    }

    /// Turns a step's answer about control into the next program counter.
    fn apply_directive(
        &mut self,
        directive: FlowDirective,
        start: usize,
        end: usize,
        prepared: &[PreparedStep<'_>],
        location: StepLocation,
    ) -> Result<StepFlow, ExecutionError> {
        match directive {
            FlowDirective::Next => Ok(StepFlow::Continue),
            FlowDirective::NotTaken => {
                self.jumps_taken = 0;
                Ok(StepFlow::Continue)
            }
            FlowDirective::Stop => Ok(StepFlow::Stop),
            FlowDirective::Jump { label, max_jumps } => {
                match find_label(&label, start, end, prepared) {
                    Some(label_index) if self.jumps_taken < max_jumps => {
                        self.jumps_taken += 1;
                        // Past the Label rather than onto it. The reference sets
                        // its counter to the Label's index and then increments,
                        // so the destination is the step after the marker.
                        Ok(StepFlow::Jump {
                            target: label_index + 1,
                        })
                    }
                    // No such label, or the allowance is spent. Both continue
                    // and both clear the counter, as the reference does -- a
                    // recipe whose jump stopped firing gets its full allowance
                    // back the next time round.
                    _ => {
                        self.jumps_taken = 0;
                        Ok(StepFlow::Continue)
                    }
                }
            }
            // Only honoured for the operation that opens a section region,
            // which `execute_region` intercepts before reaching here. Anywhere
            // else there is no region to run, and inventing one would scope the
            // rest of the recipe to a substring the recipe never asked for.
            FlowDirective::Sections { .. } => {
                Err(self.fail(ExecutionFailure::FlowDirectiveRefused, location))
            }
        }
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
