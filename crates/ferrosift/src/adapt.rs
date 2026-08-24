use alloc::string::String;

use ferrosift_model::{TextEncoding, TextValue, Value, ValueConstraint};

/// Adapts an input value to the representation the first step accepts.
///
/// Callers hold bytes far more often than they hold a typed value, and the
/// catalog splits naturally into byte-input and text-input operations. Rather
/// than making every caller write that glue, the facade performs exactly one
/// lossless conversion:
///
/// - bytes that are valid UTF-8 become UTF-8 text when text is required;
/// - UTF-8 text becomes its own bytes when bytes are required.
///
/// Anything else is passed through untouched so the executor reports the real
/// `core.executor.input_kind_mismatch` instead of this layer guessing.
pub(crate) fn to_accepted(value: Value, constraint: &ValueConstraint) -> Value {
    if constraint.accepts(value.kind()) {
        return value;
    }
    match value {
        Value::Bytes(bytes) => bytes_to_text(bytes, constraint),
        Value::Text(text) if text.encoding == TextEncoding::Utf8 => {
            Value::Bytes(text.text.into_bytes())
        }
        other => other,
    }
}

fn bytes_to_text(bytes: alloc::vec::Vec<u8>, constraint: &ValueConstraint) -> Value {
    match String::from_utf8(bytes) {
        Ok(text) if constraint.accepts(ferrosift_model::ValueKind::Text) => {
            Value::Text(TextValue {
                text,
                encoding: TextEncoding::Utf8,
            })
        }
        Ok(text) => Value::Bytes(text.into_bytes()),
        Err(error) => Value::Bytes(error.into_bytes()),
    }
}
