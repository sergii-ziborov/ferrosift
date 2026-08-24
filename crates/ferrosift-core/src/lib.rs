//! Portable operation boundary and registry for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod budget;
mod cancellation;
mod context;
mod executor;
mod operation;
mod registry;
mod trace;
mod value_size;

pub use budget::ExecutionBudget;
pub use cancellation::{Cancellation, NeverCancelled};
pub use context::OperationContext;
pub use executor::{ExecutionError, ExecutionFailure, Executor, FORK_ID, MERGE_ID, PreparedRecipe};
pub use operation::{InvalidOperationFailureCode, Operation, OperationError, OperationFailureCode};
pub use registry::{OperationRegistry, RegistryError};
pub use trace::{
    ExecutionResult, ExecutionStatus, ExecutionTrace, StepLocation, TraceEvent, TraceEventKind,
    ValueSummary,
};
