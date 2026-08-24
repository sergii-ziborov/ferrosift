//! Bounded two-phase linear recipe execution.

use alloc::vec::Vec;

use ferrosift_model::{CapabilitySet, Recipe, RecipeStep, Value};

use crate::{
    Cancellation, ExecutionBudget, ExecutionResult, OperationRegistry, StepLocation, ValueSummary,
};

mod error;
mod flow;
mod fork;
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
        let steps = preflight::resolve(recipe, self.registry, capabilities)?;
        preflight::check_runtime(&steps, input, budget, cancellation)
    }

    /// Resolves a recipe against the registry once, for repeated execution.
    ///
    /// Every check that depends only on the recipe, the registry, and the
    /// granted capabilities happens here: structural validation, operation
    /// lookup, capability checks, and argument resolution. The returned
    /// [`PreparedRecipe`] can then be executed many times without repeating
    /// any of it.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] when the recipe is invalid, names an unknown
    /// operation, requires an ungranted capability, or carries arguments the
    /// operation does not accept.
    pub fn prepare(
        &self,
        recipe: &Recipe,
        capabilities: CapabilitySet,
    ) -> Result<PreparedRecipe<'a>, ExecutionError> {
        let steps = preflight::resolve(recipe, self.registry, &capabilities)?;
        Ok(PreparedRecipe {
            steps,
            capabilities,
        })
    }

    /// Validates the complete recipe, then executes enabled steps in order.
    ///
    /// This is the one-shot path. For repeated execution use
    /// [`Executor::prepare`] once and run the result many times.
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
        self.prepare(recipe, capabilities)?
            .execute(input, budget, cancellation)
    }
}

/// A recipe already resolved against a registry.
///
/// Holding one costs the per-step registry lookups and argument resolution
/// exactly once. Each execution then applies only what genuinely depends on
/// the call: input size and representation, the budget, and cancellation.
pub struct PreparedRecipe<'a> {
    steps: Vec<preflight::PreparedStep<'a>>,
    capabilities: CapabilitySet,
}

impl PreparedRecipe<'_> {
    /// Number of steps, including disabled ones.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the recipe has no steps at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Executes the prepared steps against one input.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] when the input exceeds the budget, its
    /// representation does not flow through the steps, the run is cancelled,
    /// or a step fails.
    pub fn execute(
        &self,
        input: Value,
        budget: ExecutionBudget,
        cancellation: &dyn Cancellation,
    ) -> Result<ExecutionResult, ExecutionError> {
        preflight::check_runtime(&self.steps, &input, budget, cancellation)?;
        let initial_input_size = ValueSummary::from_value(&input).size_bytes;
        runner::run(
            &self.steps,
            input,
            budget,
            initial_input_size,
            cancellation,
            self.capabilities.clone(),
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
