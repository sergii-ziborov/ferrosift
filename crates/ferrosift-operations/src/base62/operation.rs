use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

/// The alphabet the reference offers by default.
const DEFAULT_ALPHABET: &str = "0-9A-Za-z";

fn alphabet_argument() -> ferrosift_model::ArgumentSpec {
    text_argument(
        "alphabet",
        "Digit alphabet, as a range expression.",
        DEFAULT_ALPHABET,
    )
}

/// Renders bytes as a base-62 number.
pub struct ToBase62 {
    spec: OperationSpec,
}

impl ToBase62 {
    /// Creates the Base62 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base62.encode@1",
                display_name: "To Base62",
                category: "Encoding",
                description: "Encodes bytes as a single base-62 number using a restricted, human-friendly symbol set.",
                cyberchef_alias: Some("To Base62"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![alphabet_argument()],
                inverse: Some("encoding.base62.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBase62 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase62 {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = take_bytes(input)?;
        // Empty input short-circuits before the alphabet is looked at, so a
        // malformed alphabet goes unreported on empty input. That ordering is
        // the reference's; moving the validation earlier would be tidier and
        // would disagree.
        if input.is_empty() {
            return Ok(text_output(String::new()));
        }
        let alphabet = codec::resolve(text_value(arguments, "alphabet")?)?;
        context.ensure_active()?;
        Ok(text_output(codec::encode(&input, &alphabet)))
    }
}

/// Reads a base-62 number back into bytes.
pub struct FromBase62 {
    spec: OperationSpec,
}

impl FromBase62 {
    /// Creates the Base62 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base62.decode@1",
                display_name: "From Base62",
                category: "Encoding",
                description: "Decodes a base-62 number back into bytes, ignoring characters outside the alphabet.",
                cyberchef_alias: Some("From Base62"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![alphabet_argument()],
                inverse: Some("encoding.base62.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBase62 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase62 {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = take_text(input)?;
        if input.is_empty() {
            return Ok(bytes_output(alloc::vec::Vec::new()));
        }
        let alphabet = codec::resolve(text_value(arguments, "alphabet")?)?;
        context.ensure_active()?;
        Ok(bytes_output(codec::decode(&input, &alphabet)?))
    }
}
