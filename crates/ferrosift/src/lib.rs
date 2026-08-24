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
pub use ferrosift_pattern::{EvalOptions, Node, NodeValue, Pattern, PatternError};

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

/// The names most callers want in scope.
pub mod prelude {
    pub use crate::{Arguments, Engine, Error, Pipeline, Value, pipeline};

    #[cfg(feature = "pattern")]
    pub use crate::{EvalOptions, Node, NodeValue, Pattern, parse_pattern};
}
