//! Reading an operation's input, whichever representation carries it.
//!
//! The reference passes one value between steps and *presents* it in whatever
//! type the next operation asks for — a string operation feeding a byte
//! operation converts on the way, and nothing in the recipe says so.
//! `FerroSift` used to refuse that: `To Base64` twice in a row is an ordinary
//! recipe there and was `input_kind_mismatch` here.
//!
//! So a byte-reading operation accepts text and a text-reading operation
//! accepts bytes, and this module is where the conversion happens. Two things
//! keep it from being a silent coercion. The rule is the reference's own, from
//! [`crate::jscompat::string`], rather than a plausible one invented here — and
//! `str_to_byte_array` is *not* UTF-8 encoding, which is the trap: it takes
//! UTF-16 code units directly when every one fits in a byte, so `"é"` becomes
//! one byte and not two. And the widening is declared in each operation's
//! specification, so the catalog and the ledger both show which
//! representations a step accepts rather than leaving a caller to discover it.

use alloc::{string::String, vec::Vec};

use ferrosift_core::OperationError;
use ferrosift_model::{DecimalValue, TextEncoding, TextValue, Value, ValueKind};

use crate::jscompat::string;

/// Where an operation's bytes came from.
///
/// Named because the distinction is worth seeing at a call site: one arm cost
/// nothing and the other ran a conversion whose rule is the reference's rather
/// than Rust's.
pub(crate) enum ByteSource {
    /// The value already was bytes; ownership moved, nothing was copied.
    Native(Vec<u8>),
    /// The value was text, converted by the reference's rule.
    Converted(Vec<u8>),
}

impl ByteSource {
    /// The bytes, however they arrived.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Native(bytes) | Self::Converted(bytes) => bytes,
        }
    }
}

/// Reads an input as bytes, converting from text when that is what arrived.
///
/// # Errors
///
/// Returns an error for any representation that is neither bytes nor text;
/// those are contract violations rather than conversions.
pub(crate) fn read_bytes(input: Value) -> Result<ByteSource, OperationError> {
    match input {
        Value::Bytes(value) => Ok(ByteSource::Native(value)),
        Value::Text(value) => Ok(ByteSource::Converted(string::str_to_byte_array(
            &value.text,
        ))),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Unwraps a text input, converting from bytes when that is what arrived.
pub(crate) fn take_text(input: Value) -> Result<String, OperationError> {
    match input {
        Value::Text(value) => Ok(value.text),
        Value::Bytes(value) => Ok(string::byte_array_to_utf8(&value)),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Unwraps a byte input, converting from text when that is what arrived.
pub(crate) fn take_bytes(input: Value) -> Result<Vec<u8>, OperationError> {
    read_bytes(input).map(ByteSource::into_bytes)
}

/// Reads a text input with its encoding label intact.
///
/// Bytes converted here are labelled UTF-8, which is what the conversion
/// produced rather than a guess about what they were: the reference's
/// `byteArrayToUtf8` falls back to a byte-per-character reading when the bytes
/// are not valid UTF-8, and either way the result is a Rust string.
pub(crate) fn take_text_value(input: Value) -> Result<TextValue, OperationError> {
    match input {
        Value::Text(value) => Ok(value),
        Value::Bytes(value) => Ok(TextValue {
            text: string::byte_array_to_utf8(&value),
            encoding: TextEncoding::Utf8,
        }),
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

/// Unwraps a decimal input, reading one from bytes or text when that arrived.
///
/// The runner adapts a value to the declared constraint before an operation
/// sees it, so this is usually a move rather than a conversion. Where it does
/// convert, it goes through the model's own projection -- which is the
/// reference's dish, and therefore *reads* rather than refusing: the dish
/// catches the constructor's exception and substitutes not-a-number.
pub(crate) fn take_decimal(input: Value) -> Result<DecimalValue, OperationError> {
    match input.reinterpret(ValueKind::Decimal) {
        Some(Value::Decimal(value)) => Ok(value),
        _ => Err(OperationError::InvalidArguments),
    }
}
