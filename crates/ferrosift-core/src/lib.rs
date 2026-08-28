//! Portable operation boundary and registry for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod budget;
mod cancellation;
mod context;
mod executor;
mod flow;
mod operation;
mod registry;
mod streaming;
mod trace;
mod value_size;

pub use budget::ExecutionBudget;
pub use cancellation::{Cancellation, NeverCancelled};
pub use context::OperationContext;
pub use executor::{
    CONDITIONAL_JUMP_ID, ExecutionError, ExecutionFailure, Executor, FORK_ID, JUMP_ID, LABEL_ID,
    MERGE_ID, PreparedRecipe, RETURN_ID, SUBSECTION_ID,
};
pub use flow::{FlowDirective, Section};
pub use operation::{InvalidOperationFailureCode, Operation, OperationError, OperationFailureCode};
pub use registry::{OperationRegistry, RegistryError};
pub use streaming::{CollectSink, StreamSession, StreamSink, Streamable, drive};
pub use trace::{
    ExecutionResult, ExecutionStatus, ExecutionTrace, StepLocation, TraceEvent, TraceEventKind,
    ValueSummary,
};
