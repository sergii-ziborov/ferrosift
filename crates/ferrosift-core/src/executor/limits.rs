//! Deterministic executor-owned resource checks.

use ferrosift_model::OutputBehavior;

use crate::{ExecutionBudget, ExecutionFailure};

pub(super) fn check_initial(
    step_count: usize,
    input_size: u64,
    budget: ExecutionBudget,
) -> Result<(), ExecutionFailure> {
    if step_count > budget.max_steps {
        return Err(ExecutionFailure::StepLimitExceeded);
    }
    if input_size > budget.max_input_bytes {
        return Err(ExecutionFailure::InputLimitExceeded);
    }
    Ok(())
}

/// Bounds one step's output.
///
/// The absolute output limit applies to every operation without exception.
/// The expansion *ratio* applies only where a ratio means something: an
/// operation that declares [`OutputBehavior::InputIndependent`] generates from
/// its arguments, so dividing its output by an input it never read would
/// measure nothing and refuse everything — the empty input that a generator is
/// most naturally called with makes the denominator one.
///
/// That is a narrow exemption from one check, not from the budget. An
/// operation cannot opt out of `max_output_bytes`, out of the total-bytes
/// accounting, or out of cancellation, and the default behaviour keeps the
/// ratio for everything that has not said otherwise.
pub(super) fn check_output(
    output_size: u64,
    step_input_size: u64,
    initial_input_size: u64,
    budget: ExecutionBudget,
    behavior: OutputBehavior,
) -> Result<(), ExecutionFailure> {
    if output_size > budget.max_output_bytes {
        return Err(ExecutionFailure::OutputLimitExceeded);
    }
    if matches!(behavior, OutputBehavior::InputIndependent) {
        return Ok(());
    }
    // Stated through the shared ceiling so an operation can ask the same
    // question before it builds anything. The ceiling folds in the absolute
    // limit as well, which changes nothing here: the branch above has already
    // returned for anything above it, so what is left is the ratio alone.
    if output_size > budget.output_ceiling(step_input_size)
        || output_size > budget.output_ceiling(initial_input_size)
    {
        return Err(ExecutionFailure::ExpansionRatioExceeded);
    }
    Ok(())
}
