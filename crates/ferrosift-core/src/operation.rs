//! Portable operation execution contract.

use alloc::string::String;
use core::fmt;

use ferrosift_model::{Arguments, OperationSpec, Value};

use crate::OperationContext;

const MAX_FAILURE_CODE_BYTES: usize = 128;

/// A validated namespaced code for an operation-specific failure.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationFailureCode(String);

impl OperationFailureCode {
    /// Creates a lowercase namespaced code such as `encoding.invalid_padding`.
    ///
    /// Codes contain at most 128 ASCII bytes and at least two dot-separated
    /// segments. Every segment starts with a lowercase ASCII letter and then
    /// uses lowercase letters, digits, underscores, or hyphens.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOperationFailureCode`] when the grammar is not satisfied.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidOperationFailureCode> {
        let value = value.into();
        if is_valid_failure_code(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidOperationFailureCode)
        }
    }

    /// Borrows the validated code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationFailureCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure returned when an operation-specific error code is malformed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidOperationFailureCode;

impl InvalidOperationFailureCode {
    /// Returns the stable machine-readable failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        "core.operation.failure_code_invalid"
    }
}

impl fmt::Display for InvalidOperationFailureCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl core::error::Error for InvalidOperationFailureCode {}

/// A host-independent operation implementation.
pub trait Operation: Send + Sync {
    /// Returns the complete machine-readable contract for this operation.
    fn spec(&self) -> &OperationSpec;

    /// Executes the operation against portable values and explicit ambient state.
    ///
    /// # Errors
    ///
    /// Returns an [`OperationError`] without relying on host-specific error types.
    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError>;
}

/// Stable failure surface shared by operation implementations and executors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationError {
    /// Supplied arguments do not satisfy the operation contract.
    InvalidArguments,
    /// Cooperative cancellation was requested.
    Cancelled,
    /// Output would exceed an explicit execution ceiling.
    OutputLimitExceeded,
    /// Operation-specific failure identified by a stable code.
    Failed {
        /// Namespaced stable code owned by the operation.
        code: OperationFailureCode,
    },
}

impl OperationError {
    /// Returns a stable machine-readable failure code.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::InvalidArguments => "core.operation.invalid_arguments",
            Self::Cancelled => "core.operation.cancelled",
            Self::OutputLimitExceeded => "core.operation.output_limit_exceeded",
            Self::Failed { code } => code.as_str(),
        }
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl core::error::Error for OperationError {}

fn is_valid_failure_code(value: &str) -> bool {
    if value.len() > MAX_FAILURE_CODE_BYTES {
        return false;
    }
    let mut segments = value.split('.');
    let Some(namespace) = segments.next() else {
        return false;
    };
    let Some(name) = segments.next() else {
        return false;
    };

    is_valid_code_segment(namespace)
        && is_valid_code_segment(name)
        && segments.all(is_valid_code_segment)
}

fn is_valid_code_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}
