//! Portable operation execution contract.

use alloc::{borrow::Cow, string::String};
use core::fmt;

use ferrosift_model::{Arguments, OperationSpec, Value};

use crate::OperationContext;

const MAX_FAILURE_CODE_BYTES: usize = 128;

/// A validated namespaced code for an operation-specific failure.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationFailureCode(Cow<'static, str>);

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
            Ok(Self(Cow::Owned(value)))
        } else {
            Err(InvalidOperationFailureCode)
        }
    }

    /// Creates a failure code from a built-in static value.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not follow the failure-code grammar.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        assert!(is_valid_failure_code(value), "invalid static failure code");
        Self(Cow::Borrowed(value))
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
    /// The operation would allocate more than the budget allows *while*
    /// running, regardless of how large its answer is.
    ///
    /// Separate from [`Self::OutputLimitExceeded`] because it is a different
    /// fact about a different quantity: scrypt with a large cost parameter
    /// produces a thirty-two byte key and asks for gigabytes on the way there,
    /// and an output limit that passed it would be answering the wrong
    /// question.
    TransientLimitExceeded,
    /// The operation would perform more work than the budget allows.
    ///
    /// The one ceiling that is about time rather than memory. A key derivation
    /// is *designed* to be slow and takes its cost from an argument, so the
    /// only thing standing between a recipe and an hour of CPU is a bound on
    /// the work it declares before starting.
    WorkLimitExceeded,
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
            Self::TransientLimitExceeded => "core.operation.transient_limit_exceeded",
            Self::WorkLimitExceeded => "core.operation.work_limit_exceeded",
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

const fn is_valid_failure_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() > MAX_FAILURE_CODE_BYTES || bytes.is_empty() {
        return false;
    }
    let mut segments = 1;
    let mut segment_start = true;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'.' {
            if segment_start {
                return false;
            }
            segments += 1;
            segment_start = true;
        } else if segment_start {
            if !is_lowercase(byte) {
                return false;
            }
            segment_start = false;
        } else if !is_lowercase(byte) && !is_digit(byte) && byte != b'_' && byte != b'-' {
            return false;
        }
        index += 1;
    }
    segments >= 2 && !segment_start
}

const fn is_lowercase(byte: u8) -> bool {
    byte >= b'a' && byte <= b'z'
}

const fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}
