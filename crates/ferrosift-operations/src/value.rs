//! Value unwrapping and wrapping shared by the operation implementations.
//!
//! Every operation declares an exact input kind, so a mismatch is a contract
//! violation rather than a data error; these helpers keep that single
//! judgement in one place instead of repeating it per operation.

use alloc::{string::String, vec::Vec};

use ferrosift_core::OperationError;
use ferrosift_model::{TextEncoding, TextValue, Value};

/// Unwraps a text input, rejecting anything else.
pub(crate) fn take_text(input: Value) -> Result<String, OperationError> {
    match input {
        Value::Text(value) => Ok(value.text),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Unwraps a byte input, rejecting anything else.
pub(crate) fn take_bytes(input: Value) -> Result<Vec<u8>, OperationError> {
    match input {
        Value::Bytes(value) => Ok(value),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Wraps UTF-8 text as an output value.
pub(crate) fn text(value: String) -> Value {
    Value::Text(TextValue {
        text: value,
        encoding: TextEncoding::Utf8,
    })
}

/// Wraps bytes as an output value.
pub(crate) const fn bytes(value: Vec<u8>) -> Value {
    Value::Bytes(value)
}
