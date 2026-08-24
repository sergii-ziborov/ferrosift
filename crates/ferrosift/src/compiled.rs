use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{CapabilitySet, Recipe, TextEncoding, TextValue, Value, ValueConstraint};
#[cfg(feature = "pattern")]
use ferrosift_pattern::{EvalOptions, Node};

use crate::adapt;
use crate::error::Error;

/// A pipeline already resolved against an [`crate::Engine`].
///
/// Compiling resolves every operation ID, validates the arguments, and builds
/// the recipe once. Running then executes directly: no registry construction,
/// no name lookup, and no recipe rebuild per call.
pub struct CompiledPipeline<'a> {
    registry: &'a OperationRegistry,
    recipe: Recipe,
    budget: ExecutionBudget,
    /// Input representation the first step accepts, captured at compile time
    /// so each run adapts without consulting the registry again.
    first_input: Option<ValueConstraint>,
}

impl<'a> CompiledPipeline<'a> {
    pub(crate) const fn new(
        registry: &'a OperationRegistry,
        recipe: Recipe,
        budget: ExecutionBudget,
        first_input: Option<ValueConstraint>,
    ) -> Self {
        Self {
            registry,
            recipe,
            budget,
            first_input,
        }
    }

    /// Runs the compiled pipeline and returns the final value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Execution`] when a step fails or a budget is exceeded.
    pub fn run(&self, input: Value) -> Result<Value, Error> {
        let input = match &self.first_input {
            Some(constraint) => adapt::to_accepted(input, constraint),
            None => input,
        };
        let result = Executor::new(self.registry).execute(
            &self.recipe,
            input,
            self.budget,
            &NeverCancelled,
            CapabilitySet::new(),
        )?;
        Ok(result.value)
    }

    /// Runs over bytes and returns bytes.
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run`], plus [`Error::UnexpectedOutput`] when the
    /// final value is neither bytes nor UTF-8 text.
    pub fn run_bytes(&self, input: &[u8]) -> Result<Vec<u8>, Error> {
        match self.run(Value::Bytes(input.to_vec()))? {
            Value::Bytes(bytes) => Ok(bytes),
            Value::Text(text) if text.encoding == TextEncoding::Utf8 => Ok(text.text.into_bytes()),
            _ => Err(Error::UnexpectedOutput),
        }
    }

    /// Runs over text and returns text.
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run`], plus [`Error::UnexpectedOutput`] when the
    /// final value is not UTF-8 text.
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

    /// Runs the transforms, then evaluates a hex pattern over the result.
    #[cfg(feature = "pattern")]
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run_bytes`], plus [`Error::Pattern`] when the
    /// pattern fails to parse or evaluate.
    pub fn run_pattern(&self, source: &str, input: &[u8]) -> Result<Vec<Node>, Error> {
        self.run_pattern_with(source, input, &EvalOptions::default())
    }

    /// [`CompiledPipeline::run_pattern`] with explicit evaluation bounds.
    #[cfg(feature = "pattern")]
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run_pattern`].
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

    /// The resolved recipe, for export or inspection.
    #[must_use]
    pub const fn recipe(&self) -> &Recipe {
        &self.recipe
    }
}
