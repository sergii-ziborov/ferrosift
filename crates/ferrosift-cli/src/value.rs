//! Explicit CLI value representations.

use ferrosift_model::{TextEncoding, TextValue, Value};

use crate::{args::InputKind, error::CliError};

pub fn empty(kind: InputKind) -> Value {
    match kind {
        InputKind::Bytes => Value::Bytes(Vec::new()),
        InputKind::Text => Value::Text(TextValue {
            text: String::new(),
            encoding: TextEncoding::Utf8,
        }),
    }
}

pub fn input(bytes: Vec<u8>, kind: InputKind) -> Result<Value, CliError> {
    match kind {
        InputKind::Bytes => Ok(Value::Bytes(bytes)),
        InputKind::Text => String::from_utf8(bytes)
            .map(|text| {
                Value::Text(TextValue {
                    text,
                    encoding: TextEncoding::Utf8,
                })
            })
            .map_err(|error| CliError::new("cli.input.invalid_utf8", error.to_string())),
    }
}

pub fn output(value: Value) -> Result<Vec<u8>, CliError> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }) => Ok(text.into_bytes()),
        Value::Text(_) => Err(CliError::new(
            "cli.output.encoding_unsupported",
            "only UTF-8 text output is supported",
        )),
        other => Err(CliError::new(
            "cli.output.kind_unsupported",
            format!("kind={}", other.kind()),
        )),
    }
}
