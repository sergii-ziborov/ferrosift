use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ferrosift_core::{ExecutionBudget, NeverCancelled, PreparedRecipe};
use ferrosift_model::{TextEncoding, TextValue, Value, ValueConstraint};
#[cfg(feature = "pattern")]
use ferrosift_pattern::{EvalOptions, Node, Pattern};

use crate::adapt;
use crate::error::Error;

/// A pipeline already resolved against an [`crate::Engine`].
///
/// Compiling builds the registry-resolved steps once: the operation lookups,
/// capability checks, and argument resolution all happen here. Running then
/// applies only what depends on the call — input size and representation, the
/// budget, and cancellation — before executing.
pub struct CompiledPipeline<'a> {
    prepared: PreparedRecipe<'a>,
    budget: ExecutionBudget,
    /// Input representation the first step accepts, captured at compile time
    /// so each run adapts without consulting the registry again.
    first_input: Option<ValueConstraint>,
}

impl<'a> CompiledPipeline<'a> {
    pub(crate) const fn new(
        prepared: PreparedRecipe<'a>,
        budget: ExecutionBudget,
        first_input: Option<ValueConstraint>,
    ) -> Self {
        Self {
            prepared,
            budget,
            first_input,
        }
    }

    /// Number of resolved steps, including disabled ones.
    #[must_use]
    pub fn len(&self) -> usize {
        self.prepared.len()
    }

    /// Whether the pipeline has no steps at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prepared.is_empty()
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
        let result = self.prepared.execute(input, self.budget, &NeverCancelled)?;
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

    #[cfg(feature = "pattern")]
    /// Runs the transforms, then evaluates a hex pattern over the result.
    ///
    /// This parses `source` on every call. When the same pattern is applied
    /// repeatedly, parse it once with [`crate::parse_pattern`] and use
    /// [`CompiledPipeline::run_parsed`] instead.
    ///
    /// ```
    /// use ferrosift::{Engine, NodeValue};
    ///
    /// let engine = Engine::portable()?;
    /// let pipeline = engine.pipeline().from_base64().compile(&engine)?;
    ///
    /// // "Q0FGRQ==" is Base64 for the ASCII bytes "CAFE".
    /// let nodes = pipeline.run_pattern("be u16 magic @ 0x00;", b"Q0FGRQ==")?;
    /// assert_eq!(nodes[0].value, NodeValue::Unsigned(0x4341));
    /// # Ok::<(), ferrosift::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run_bytes`], plus [`Error::Pattern`] when the
    /// pattern fails to parse or evaluate.
    pub fn run_pattern(&self, source: &str, input: &[u8]) -> Result<Vec<Node>, Error> {
        self.run_pattern_with(source, input, &EvalOptions::default())
    }

    #[cfg(feature = "pattern")]
    /// Runs the transforms, then evaluates an already-parsed pattern.
    ///
    /// This is the repeated-analysis path: the transforms are resolved once by
    /// [`crate::Pipeline::compile`] and the pattern is parsed once by
    /// [`crate::parse_pattern`], leaving each call to transform and evaluate.
    ///
    /// ```
    /// use ferrosift::{Engine, NodeValue};
    ///
    /// let engine = Engine::portable()?;
    /// let pipeline = engine.pipeline().from_base64().compile(&engine)?;
    /// let pattern = ferrosift::parse_pattern("be u16 magic @ 0x00;")?;
    ///
    /// for encoded in [&b"Q0FGRQ=="[..], b"REVBRA=="] {
    ///     let nodes = pipeline.run_parsed(&pattern, encoded)?;
    ///     assert!(matches!(nodes[0].value, NodeValue::Unsigned(_)));
    /// }
    /// # Ok::<(), ferrosift::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run_bytes`], plus [`Error::Pattern`] when the
    /// pattern does not evaluate against the transformed bytes.
    pub fn run_parsed(&self, pattern: &Pattern, input: &[u8]) -> Result<Vec<Node>, Error> {
        self.run_parsed_with(pattern, input, &EvalOptions::default())
    }

    #[cfg(feature = "pattern")]
    /// [`CompiledPipeline::run_parsed`] with explicit evaluation bounds.
    ///
    /// # Errors
    ///
    /// As [`CompiledPipeline::run_parsed`].
    pub fn run_parsed_with(
        &self,
        pattern: &Pattern,
        input: &[u8],
        options: &EvalOptions,
    ) -> Result<Vec<Node>, Error> {
        let bytes = self.run_bytes(input)?;
        Ok(ferrosift_pattern::evaluate(pattern, &bytes, options)?)
    }

    #[cfg(feature = "pattern")]
    /// [`CompiledPipeline::run_pattern`] with explicit evaluation bounds.
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

    /// The resolved steps, for inspection.
    #[must_use]
    pub const fn prepared(&self) -> &PreparedRecipe<'a> {
        &self.prepared
    }
}
