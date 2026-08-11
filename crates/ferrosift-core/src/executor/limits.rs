//! Deterministic executor-owned resource checks.

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

pub(super) fn check_output(
    output_size: u64,
    step_input_size: u64,
    initial_input_size: u64,
    budget: ExecutionBudget,
) -> Result<(), ExecutionFailure> {
    if output_size > budget.max_output_bytes {
        return Err(ExecutionFailure::OutputLimitExceeded);
    }
    if exceeds_ratio(output_size, step_input_size, budget.max_expansion_ratio)
        || exceeds_ratio(output_size, initial_input_size, budget.max_expansion_ratio)
    {
        return Err(ExecutionFailure::ExpansionRatioExceeded);
    }
    Ok(())
}

fn exceeds_ratio(output_size: u64, input_size: u64, ratio: u32) -> bool {
    let denominator = input_size.max(1);
    output_size > denominator.saturating_mul(u64::from(ratio))
}
