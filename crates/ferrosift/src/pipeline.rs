use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_core::{ExecutionBudget, OperationRegistry};
use ferrosift_model::{
    Arguments, OperationId, Recipe, RecipeMetadata, RecipeStep, StepId, TextEncoding, TextValue,
    Value, ValueConstraint,
};
#[cfg(feature = "pattern")]
use ferrosift_pattern::{EvalOptions, Node};

use crate::compiled::CompiledPipeline;
use crate::engine::Engine;
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

    /// Resolves this pipeline against an engine, once.
    ///
    /// Compiling does every lookup and validation up front, so the returned
    /// [`CompiledPipeline`] can be run repeatedly without rebuilding the
    /// registry or the recipe. Prefer this whenever the same pipeline runs
    /// more than once.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when an operation is unknown or the steps do not form
    /// a valid recipe.
    pub fn compile<'a>(&self, engine: &'a Engine) -> Result<CompiledPipeline<'a>, Error> {
        let registry = engine.registry();
        let recipe = self.recipe(registry)?;
        Ok(CompiledPipeline::new(
            registry,
            recipe,
            self.budget,
            self.first_input(registry),
        ))
    }

    /// Runs the pipeline and returns the final value.
    ///
    /// This is the one-shot convenience path: it builds a whole engine for
    /// this single call. For repeated execution use [`Pipeline::compile`]
    /// against a reused [`Engine`] instead.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when an operation is unknown, the recipe is not
    /// valid, or execution fails or exceeds a budget.
    pub fn run(&self, input: Value) -> Result<Value, Error> {
        let engine = Engine::portable()?;
        self.compile(&engine)?.run(input)
    }

    /// The input representation the first step accepts, if it has one.
    fn first_input(&self, registry: &OperationRegistry) -> Option<ValueConstraint> {
        let (operation, _) = self.steps.first()?;
        let id = OperationId::new(operation.as_str()).ok()?;
        registry
            .get(&id)
            .map(|operation| operation.spec().input.clone())
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
    #[cfg(feature = "pattern")]
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
    #[cfg(feature = "pattern")]
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
