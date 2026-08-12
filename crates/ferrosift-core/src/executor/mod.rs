//! Bounded two-phase linear recipe execution.

use ferrosift_model::{CapabilitySet, Recipe, RecipeStep, Value};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, OperationRegistry, StepLocation, ValueSummary,
};

mod error;
mod flow;
mod limits;
mod preflight;
mod runner;

pub use error::{ExecutionError, ExecutionFailure};
/// Flow-control operation identifiers used by the executor.
pub use flow::{FORK_ID, MERGE_ID};

/// Portable executor for validated linear recipes.
pub struct Executor<'a> {
    registry: &'a OperationRegistry,
}

impl<'a> Executor<'a> {
    /// Creates an executor backed by a validated operation registry.
    #[must_use]
    pub const fn new(registry: &'a OperationRegistry) -> Self {
        Self { registry }
    }

    /// Validates a complete recipe without invoking any operation.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] for the same model, budget, registry,
    /// capability, argument, cancellation, and cross-step type failures checked
    /// before [`Self::execute`].
    pub fn validate(
        &self,
        recipe: &Recipe,
        input: &Value,
        budget: ExecutionBudget,
        cancellation: &dyn Cancellation,
        capabilities: &CapabilitySet,
    ) -> Result<(), ExecutionError> {
        preflight::prepare(
            recipe,
            self.registry,
            input,
            budget,
            cancellation,
            capabilities,
        )
        .map(drop)
    }

    /// Validates the complete recipe, then executes enabled steps in order.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] before any invocation when preflight fails, or
    /// with the exact responsible step and bounded partial trace at runtime.
    pub fn execute(
        &self,
        recipe: &Recipe,
        input: Value,
        budget: ExecutionBudget,
        cancellation: &dyn Cancellation,
        capabilities: CapabilitySet,
    ) -> Result<ExecutionResult, ExecutionError> {
        let initial_input_size = ValueSummary::from_value(&input).size_bytes;
        let prepared = preflight::prepare(
            recipe,
            self.registry,
            &input,
            budget,
            cancellation,
            &capabilities,
        )?;
        runner::run(
            &prepared,
            input,
            budget,
            initial_input_size,
            cancellation,
            capabilities,
        )
    }
}

fn step_location(index: usize, step: &RecipeStep) -> StepLocation {
    StepLocation {
        index,
        step_id: step.id.clone(),
        operation: step.operation.clone(),
    }
}
