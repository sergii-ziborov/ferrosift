//! Subsection region driving for the region interpreter.
//!
//! A Subsection is a Fork that iterates spans instead of branches. It splits
//! the value at offsets someone else chose, runs the same body on each piece,
//! and puts the pieces back where they came from -- so the text between the
//! spans survives untouched, which is the whole difference from a Fork.
//!
//! Nothing here knows what selected the spans. The operation answers with byte
//! ranges (see [`FlowDirective::Sections`]) and the executor slices, which is
//! what keeps the regular expression engine in `ferrosift-operations` and out
//! of a `no_std` core.

use alloc::string::String;
use core::mem;

use ferrosift_model::{OutputBehavior, TextEncoding, TextValue, Value};

use crate::{
    FlowDirective, Operation, OperationContext, OperationError, Section, StepLocation, TraceEvent,
    TraceEventKind, ValueSummary,
};

use super::preflight::PreparedStep;
use super::runner::{Runner, StepControl, StepFlow, after_merge};
use super::{ExecutionError, ExecutionFailure, limits};

/// Where a region starts, where its Merge is, and where the enclosing one ends.
pub(super) struct Region {
    /// Index of the step that opened the region.
    open: usize,
    /// Index of the matching Merge, clamped into the enclosing region.
    merge: usize,
    /// One past the last step the enclosing region owns.
    end: usize,
}

impl Runner<'_> {
    pub(super) fn run_subsection(
        &mut self,
        index: usize,
        merge_index: usize,
        region_end: usize,
        prepared: &[PreparedStep<'_>],
    ) -> Result<StepFlow, ExecutionError> {
        let step = &prepared[index];
        let location = step.location(index);
        if self.cancellation.is_cancelled() {
            return Err(self.fail(
                ExecutionFailure::Operation(OperationError::Cancelled),
                location,
            ));
        }
        if step.breakpoint {
            self.trace.events.push(TraceEvent {
                location,
                kind: TraceEventKind::BreakpointReached {
                    input: ValueSummary::from_value(&self.value),
                },
            });
            return Ok(StepFlow::Pause { step_index: index });
        }
        let Some(operation) = step.operation else {
            return Err(self.fail(ExecutionFailure::UnknownOperation, location));
        };

        self.enter_flow(&location)?;
        self.count_invocation(&location)?;
        let input_summary = ValueSummary::from_value(&self.value);
        self.trace.events.push(TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted {
                input: input_summary,
            },
        });
        let region = Region {
            open: index,
            merge: merge_index.min(region_end),
            end: region_end,
        };
        let outcome = self.drive_sections(operation, &region, prepared, &location);
        self.leave_flow();

        match outcome? {
            Sectioned::Pause { step_index } => Ok(StepFlow::Pause { step_index }),
            // Nothing selected. The reference returns its state untouched here
            // and the following steps run on the whole value, so this is a
            // fall-through and not an empty region.
            Sectioned::Fallthrough(output) => {
                self.trace.events.push(TraceEvent {
                    location,
                    kind: TraceEventKind::StepCompleted {
                        output: ValueSummary::from_value(&output),
                    },
                });
                self.value = output;
                Ok(StepFlow::Continue)
            }
            Sectioned::Spliced(output) => {
                let output_summary = ValueSummary::from_value(&output);
                // The same instrument a Fork's join is measured with, for the
                // same reason: the answer is the input with each span
                // rewritten, so growth here is growth of the input and the
                // ratio is the right question to ask about it.
                if let Err(failure) = limits::check_output(
                    output_summary.size_bytes,
                    input_summary.size_bytes,
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
                self.emit_merge_events(region.merge, region.end, prepared, &output)?;
                self.value = output;
                Ok(StepFlow::Jump {
                    target: after_merge(region.merge, region.end, prepared),
                })
            }
        }
    }

    fn drive_sections(
        &mut self,
        operation: &dyn Operation,
        region: &Region,
        prepared: &[PreparedStep<'_>],
        location: &StepLocation,
    ) -> Result<Sectioned, ExecutionError> {
        // Read as text *before* asking, so the offsets the operation reports
        // and the string this slices are the same string. Asking first and
        // converting after would leave the two free to disagree about where a
        // byte is.
        let input = super::fork::value_as_text(mem::replace(&mut self.value, Value::Empty))
            .map_err(|failure| self.fail(failure, location.clone()))?;
        let seen = Value::Text(TextValue {
            text: input,
            encoding: TextEncoding::Utf8,
        });
        let context =
            OperationContext::new(self.budget, self.cancellation, self.capabilities.clone());
        let arguments = &prepared[region.open].arguments;
        let directive = operation
            .direct(&seen, arguments, &context)
            .map_err(|error| self.fail(ExecutionFailure::Operation(error), location.clone()))?;
        let Value::Text(input) = seen else {
            unreachable!("the value was just built as text")
        };
        let input = input.text;

        let (spans, ignore_errors) = match directive {
            FlowDirective::Sections {
                spans,
                ignore_errors,
            } => (spans, ignore_errors),
            FlowDirective::Next => {
                return Ok(Sectioned::Fallthrough(Value::Text(TextValue {
                    text: input,
                    encoding: TextEncoding::Utf8,
                })));
            }
            _ => {
                return Err(self.fail(ExecutionFailure::FlowDirectiveRefused, location.clone()));
            }
        };
        if let Err(failure) = check_spans(&spans, &input) {
            return Err(self.fail(failure, location.clone()));
        }
        if spans.len() > self.budget.max_branches {
            return Err(self.fail(ExecutionFailure::BranchLimitExceeded, location.clone()));
        }
        self.map_sections(&input, &spans, region, ignore_errors, prepared, location)
    }

    fn map_sections(
        &mut self,
        input: &str,
        spans: &[Section],
        region: &Region,
        ignore_errors: bool,
        prepared: &[PreparedStep<'_>],
        location: &StepLocation,
    ) -> Result<Sectioned, ExecutionError> {
        let mut output = String::with_capacity(input.len());
        let mut offset = 0_usize;
        for span in spans {
            if self.cancellation.is_cancelled() {
                return Err(self.fail(
                    ExecutionFailure::Operation(OperationError::Cancelled),
                    location.clone(),
                ));
            }
            // Everything between the previous span and this one is carried
            // through untouched. That is the difference between scoping a
            // recipe and splitting a value: a Fork keeps only what it re-joins.
            output.push_str(&input[offset..span.start]);
            let section = &input[span.start..span.end];
            self.account_bytes(section.len() as u64, location)?;
            self.value = Value::Text(TextValue {
                text: String::from(section),
                encoding: TextEncoding::Utf8,
            });
            let outcome = self.in_nested_recipe(|runner| {
                runner.execute_region(region.open + 1, region.merge, prepared)
            });
            match outcome {
                // A `Return` inside a tranche ends that tranche, not the run:
                // the reference gives each one its own recipe to return from.
                Ok(StepControl::Continue | StepControl::Stop) => {
                    match super::fork::value_as_text(mem::replace(&mut self.value, Value::Empty)) {
                        Ok(text) => {
                            self.account_bytes(text.len() as u64, location)?;
                            output.push_str(&text);
                        }
                        Err(_) if ignore_errors => {}
                        Err(failure) => return Err(self.fail(failure, location.clone())),
                    }
                }
                Ok(StepControl::Pause { step_index }) => {
                    return Ok(Sectioned::Pause { step_index });
                }
                // A failing tranche contributes nothing, the way a failing Fork
                // branch does. The reference splices its error *message* in
                // instead, which is a debugging aid there and would be an
                // injection of unrelated text into the answer here.
                Err(_) if ignore_errors => self.value = Value::Empty,
                Err(error) => return Err(error),
            }
            offset = span.end;
        }
        output.push_str(&input[offset..]);
        Ok(Sectioned::Spliced(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        })))
    }
}

enum Sectioned {
    /// The pattern selected nothing to scope; the value is unchanged.
    Fallthrough(Value),
    /// The region ran on every span and the pieces were put back.
    Spliced(Value),
    Pause {
        step_index: usize,
    },
}

/// Refuses spans that do not describe a run of whole characters, in order.
///
/// The operation builds these from matches over the very string this checks
/// against, so a violation means a bug rather than a bad recipe -- and a bug
/// that would otherwise be a slice panic on a character boundary.
fn check_spans(spans: &[Section], input: &str) -> Result<(), ExecutionFailure> {
    let mut previous = 0_usize;
    for span in spans {
        let ordered = span.fits(input.len()) && span.start >= previous;
        let aligned = input.is_char_boundary(span.start) && input.is_char_boundary(span.end);
        if !ordered || !aligned {
            return Err(ExecutionFailure::FlowDirectiveRefused);
        }
        previous = span.end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Section, check_spans};

    #[test]
    fn spans_must_be_ordered_and_aligned() {
        let input = "aé b";
        assert!(check_spans(&[Section::new(0, 1), Section::new(3, 4)], input).is_ok());
        // Overlapping.
        assert!(check_spans(&[Section::new(0, 3), Section::new(1, 4)], input).is_err());
        // Past the end.
        assert!(check_spans(&[Section::new(0, 9)], input).is_err());
        // Inside a two-byte character.
        assert!(check_spans(&[Section::new(1, 2)], input).is_err());
        // Reversed.
        assert!(check_spans(&[Section::new(3, 1)], input).is_err());
    }
}
