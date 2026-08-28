//! One API for `FerroSift` data transformations and binary patterns.
//!
//! This crate is the facade over the `FerroSift` workspace: a pipeline
//! builder over the verified operation catalog, the hex-pattern engine, and a
//! single error type with stable codes, so callers do not glue several error
//! models together.
//!
//! The signature move is transform-then-parse: decode, decompress, or decrypt
//! a buffer and describe the bytes that come out, in one call.
//!
//! That path lives on [`CompiledPipeline::run_pattern`], whose documentation
//! carries the worked example. It needs the `pattern` feature, so the example
//! lives there rather than here, where it would break a build without it.
//!
//! Every pipeline is bounded by construction. Input, output, expansion, step
//! count, and pattern nesting all have limits that callers may raise or lower
//! but never remove.
//!
//! ```
//! let text = ferrosift::pipeline().to_hex().run_text("Hi")?;
//! assert_eq!(text, "48 69");
//! # Ok::<(), ferrosift::Error>(())
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod adapt;
mod compiled;
mod engine;
mod error;
mod pipeline;
mod steps;

pub use compiled::CompiledPipeline;
pub use engine::Engine;
pub use error::Error;
pub use pipeline::{Pipeline, default_budget, pipeline, registry};

pub use ferrosift_core::{
    ExecutionBudget, ExecutionError, Operation, OperationRegistry, PreparedRecipe,
};
pub use ferrosift_model::{
    ArgumentValue, Arguments, Recipe, StructuredValue, TextEncoding, TextValue, Value,
};
#[cfg(feature = "pattern")]
pub use ferrosift_pattern::{
    ByteSource, EvalOptions, Node, NodeValue, Pattern, PatternError, SourceError,
};

/// The built-in operations, for assembling a catalog smaller than the whole.
///
/// [`Engine::portable`] builds every operation, which is the right default and
/// the wrong thing for a deployment that wants three of them. Registering only
/// what is needed is a supported way to use this library, not a workaround:
///
/// ```
/// use ferrosift::{Engine, OperationRegistry, operations};
///
/// let mut registry = OperationRegistry::new();
/// registry.declare_evidence(operations::evidence_manifest())?;
/// registry.register(operations::FromBase64::new())?;
/// let engine = Engine::with_registry(registry);
///
/// assert_eq!(engine.len(), 1);
/// # Ok::<(), ferrosift::Error>(())
/// ```
///
/// The smaller catalog is not only a smaller registry. Nothing then references
/// [`ferrosift_operations::default_registry`], so the operations left out can
/// be dropped from the binary entirely rather than merely going unregistered.
pub use ferrosift_operations as operations;

/// Parses a hex pattern once, for repeated evaluation.
///
/// Pair with [`CompiledPipeline::run_parsed`] so a loop over many inputs
/// neither re-resolves the transforms nor re-parses the pattern.
///
/// # Errors
///
/// Returns [`Error::Pattern`] when the source does not lex or parse.
#[cfg(feature = "pattern")]
pub fn parse_pattern(source: &str) -> Result<Pattern, Error> {
    Ok(ferrosift_pattern::parse(source)?)
}

/// Evaluates a parsed pattern against any [`ByteSource`].
///
/// The pipeline methods take a `&[u8]` because a transform has to produce its
/// output somewhere, and that somewhere is memory. Describing bytes *without*
/// transforming them has no such constraint, which is the case a hex pattern is
/// usually for: a disk image or a firmware dump larger than the machine reading
/// it, where a pattern touches a few dozen scalars and nothing else.
///
/// ```
/// use ferrosift::{ByteSource, EvalOptions, NodeValue, SourceError};
///
/// /// A window onto something bigger, without a copy of it.
/// struct Window<'a> {
///     bytes: &'a [u8],
///     base: u64,
/// }
///
/// impl ByteSource for Window<'_> {
///     fn len(&self) -> u64 {
///         self.base + self.bytes.len() as u64
///     }
///
///     fn read_exact_at(&self, offset: u64, into: &mut [u8]) -> Result<(), SourceError> {
///         let at = offset
///             .checked_sub(self.base)
///             .and_then(|at| usize::try_from(at).ok())
///             .ok_or_else(|| SourceError::new("before the window"))?;
///         let end = at + into.len();
///         into.copy_from_slice(
///             self.bytes
///                 .get(at..end)
///                 .ok_or_else(|| SourceError::new("after the window"))?,
///         );
///         Ok(())
///     }
/// }
///
/// let pattern = ferrosift::parse_pattern("be u16 magic @ 0x10000000;")?;
/// let window = Window { bytes: &[0xca, 0xfe], base: 0x1000_0000 };
/// let nodes = ferrosift::evaluate_pattern(&pattern, &window, &EvalOptions::default())?;
/// assert_eq!(nodes[0].value, NodeValue::Unsigned(0xcafe));
/// # Ok::<(), ferrosift::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::Pattern`] when a read leaves the source, the source
/// declines a read, or the pattern exceeds the bounds in `options`.
#[cfg(feature = "pattern")]
pub fn evaluate_pattern<S: ByteSource + ?Sized>(
    pattern: &Pattern,
    source: &S,
    options: &EvalOptions,
) -> Result<alloc::vec::Vec<Node>, Error> {
    Ok(ferrosift_pattern::evaluate_with(pattern, source, options)?)
}

/// The names most callers want in scope.
pub mod prelude {
    pub use crate::{Arguments, Engine, Error, Pipeline, Value, pipeline};

    #[cfg(feature = "pattern")]
    pub use crate::{
        ByteSource, EvalOptions, Node, NodeValue, Pattern, evaluate_pattern, parse_pattern,
    };
}
