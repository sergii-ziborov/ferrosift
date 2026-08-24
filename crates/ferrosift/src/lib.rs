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
//! ```
//! use ferrosift::NodeValue;
//!
//! // "Q0FGRQ==" is Base64 for the bytes CA FE.
//! let nodes = ferrosift::pipeline()
//!     .from_base64()
//!     .run_pattern("be u16 magic @ 0x00;", b"Q0FGRQ==")?;
//!
//! assert_eq!(nodes[0].value, NodeValue::Unsigned(0x4341));
//! # Ok::<(), ferrosift::Error>(())
//! ```
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

pub use ferrosift_core::{ExecutionBudget, ExecutionError, Operation, OperationRegistry};
pub use ferrosift_model::{
    ArgumentValue, Arguments, Recipe, StructuredValue, TextEncoding, TextValue, Value,
};
#[cfg(feature = "pattern")]
pub use ferrosift_pattern::{EvalOptions, Node, NodeValue, PatternError};

/// The names most callers want in scope.
pub mod prelude {
    pub use crate::{Arguments, Engine, Error, Pipeline, Value, pipeline};

    #[cfg(feature = "pattern")]
    pub use crate::{EvalOptions, Node, NodeValue};
}
