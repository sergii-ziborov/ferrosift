use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{
    Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep, StepId,
    TextEncoding, TextValue, Value,
};
use ferrosift_pattern::{EvalOptions, Node};

use crate::error::Error;

/// The default execution budget: 16 MiB in, 64 MiB out, 256 steps.
///
/// Every pipeline is bounded by construction; callers raise or lower this
/// with [`Pipeline::budget`] rather than opting out of limits.
#[must_use]
pub const fn default_budget() -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 256,
        max_input_bytes: 16 * 1024 * 1024,
        max_output_bytes: 64 * 1024 * 1024,
        max_expansion_ratio: 512,
        max_branches: 4096,
        max_flow_depth: 16,
        max_operation_invocations: 1_000_000,
        max_total_bytes_processed: 256 * 1024 * 1024,
    }
}

/// Builds a transformation pipeline.
///
/// Steps are recorded in order and executed by the same engine that runs
/// recipes, so a pipeline and the equivalent recipe behave identically.
#[must_use]
pub fn pipeline() -> Pipeline {
    Pipeline::new()
}

/// An ordered list of transformations with an execution budget.
#[derive(Clone, Debug)]
pub struct Pipeline {
    steps: Vec<(String, Arguments)>,
    budget: ExecutionBudget,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    /// Creates an empty pipeline with [`default_budget`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            steps: Vec::new(),
            budget: default_budget(),
        }
    }

    /// Replaces the execution budget.
    #[must_use]
    pub const fn budget(mut self, budget: ExecutionBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Appends a step by canonical operation ID with explicit arguments.
    ///
    /// This is the escape hatch: every built-in operation is reachable here,
    /// including those without a typed convenience method.
    #[must_use]
    pub fn step(mut self, operation: &str, arguments: Arguments) -> Self {
        self.steps.push((operation.to_string(), arguments));
        self
    }

    /// The canonical operation IDs this pipeline will run, in order.
    #[must_use]
    pub fn operations(&self) -> Vec<&str> {
        self.steps.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// Runs the pipeline and returns the final value.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when an operation is unknown, the recipe is not
    /// valid, or execution fails or exceeds a budget.
    pub fn run(&self, input: Value) -> Result<Value, Error> {
        let registry = registry()?;
        let recipe = self.recipe(&registry)?;
        let input = self.adapt_input(&registry, input);
        let result = Executor::new(&registry).execute(
            &recipe,
            input,
            self.budget,
            &NeverCancelled,
            CapabilitySet::new(),
        )?;
        Ok(result.value)
    }

    /// Converts the caller's value once, if the first step needs the other
    /// representation. See [`crate::adapt::to_accepted`].
    fn adapt_input(&self, registry: &OperationRegistry, input: Value) -> Value {
        let Some((operation, _)) = self.steps.first() else {
            return input;
        };
        let Ok(id) = OperationId::new(operation.as_str()) else {
            return input;
        };
        registry.get(&id).map_or(input.clone(), |operation| {
            crate::adapt::to_accepted(input, &operation.spec().input)
        })
    }

    /// Runs the pipeline over bytes and returns bytes.
    ///
    /// Text output is returned as its UTF-8 encoding, so a pipeline ending in
    /// an encoder still yields bytes.
    ///
    /// # Errors
    ///
    /// As [`Pipeline::run`], plus [`Error::UnexpectedOutput`] when the final
    /// value is neither bytes nor UTF-8 text.
    pub fn run_bytes(&self, input: &[u8]) -> Result<Vec<u8>, Error> {
        match self.run(Value::Bytes(input.to_vec()))? {
            Value::Bytes(bytes) => Ok(bytes),
            Value::Text(text) if text.encoding == TextEncoding::Utf8 => Ok(text.text.into_bytes()),
            _ => Err(Error::UnexpectedOutput),
        }
    }

    /// Runs the pipeline over text and returns text.
    ///
    /// # Errors
    ///
    /// As [`Pipeline::run`], plus [`Error::UnexpectedOutput`] when the final
    /// value is not UTF-8 text.
    pub fn run_text(&self, input: &str) -> Result<String, Error> {
        let value = Value::Text(TextValue {
            text: input.to_string(),
            encoding: TextEncoding::Utf8,
        });
        match self.run(value)? {
            Value::Text(text) if text.encoding == TextEncoding::Utf8 => Ok(text.text),
            _ => Err(Error::UnexpectedOutput),
        }
    }

    /// Runs the pipeline, then evaluates a hex pattern over the result.
    ///
    /// This is the transform-then-parse path: decode, decompress, or decrypt
    /// a buffer and describe the bytes that come out, in one call.
    ///
    /// # Errors
    ///
    /// As [`Pipeline::run_bytes`], plus [`Error::Pattern`] when the pattern
    /// fails to parse or evaluate against the transformed bytes.
    pub fn run_pattern(&self, source: &str, input: &[u8]) -> Result<Vec<Node>, Error> {
        self.run_pattern_with(source, input, &EvalOptions::default())
    }

    /// [`Pipeline::run_pattern`] with explicit evaluation bounds.
    ///
    /// # Errors
    ///
    /// As [`Pipeline::run_pattern`].
    pub fn run_pattern_with(
        &self,
        source: &str,
        input: &[u8],
        options: &EvalOptions,
    ) -> Result<Vec<Node>, Error> {
        let bytes = self.run_bytes(input)?;
        let pattern = ferrosift_pattern::parse(source)?;
        Ok(ferrosift_pattern::evaluate(&pattern, &bytes, options)?)
    }

    fn recipe(&self, registry: &OperationRegistry) -> Result<Recipe, Error> {
        let mut steps = Vec::with_capacity(self.steps.len());
        for (index, (operation, arguments)) in self.steps.iter().enumerate() {
            let id = OperationId::new(operation.as_str())
                .map_err(|_| Error::UnknownOperation(operation.clone()))?;
            if registry.get(&id).is_none() {
                return Err(Error::UnknownOperation(operation.clone()));
            }
            steps.push(RecipeStep {
                id: StepId::new(format!("step-{index:04}")).map_err(|_| Error::InvalidRecipe)?,
                operation: id,
                arguments: arguments.clone(),
                disabled: false,
                breakpoint: false,
            });
        }
        Recipe::new(steps, RecipeMetadata::default()).map_err(|_| Error::InvalidRecipe)
    }
}

/// Builds the validated registry of every built-in operation.
///
/// # Errors
///
/// Returns [`Error::Registry`] if an internal contract does not validate.
pub fn registry() -> Result<OperationRegistry, Error> {
    ferrosift_operations::default_registry().map_err(Error::from)
}
