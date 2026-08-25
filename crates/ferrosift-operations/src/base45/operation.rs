use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

const STANDARD_ALPHABET: &str = "0-9A-Z $%*+\\-./:";

/// Encodes bytes as Base45 text using an explicit alphabet.
pub struct ToBase45 {
    spec: OperationSpec,
}

impl ToBase45 {
    /// Creates the Base45 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base45.encode@1",
                display_name: "To Base45",
                category: "Encoding",
                description: "Encodes bytes as Base45 text with an explicit alphabet.",
                cyberchef_alias: Some("To Base45"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "alphabet",
                    "Base45 alphabet range expression.",
                    STANDARD_ALPHABET,
                )],
                inverse: Some("encoding.base45.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBase45 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase45 {
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
        let input = crate::value::take_bytes(input)?;
        let output = if input.is_empty() {
            String::new()
        } else {
            codec::encode(&input, text_value(arguments, "alphabet")?, context)?
        };
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base45 text into bytes using an explicit alphabet.
pub struct FromBase45 {
    spec: OperationSpec,
}

impl FromBase45 {
    /// Creates the Base45 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base45.decode@1",
                display_name: "From Base45",
                category: "Encoding",
                description: "Decodes validated Base45 text into bytes.",
                cyberchef_alias: Some("From Base45"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base45 alphabet range expression.",
                        STANDARD_ALPHABET,
                    ),
                    boolean_argument(
                        "remove_non_alphabet",
                        "Remove characters outside the selected alphabet.",
                        true,
                    ),
                ],
                inverse: Some("encoding.base45.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBase45 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase45 {
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
        let input = crate::value::take_text_value(input)?;
        if input.text.is_empty() {
            return Ok(Value::Bytes(Vec::new()));
        }
        codec::decode(
            &input.text,
            text_value(arguments, "alphabet")?,
            boolean_value(arguments, "remove_non_alphabet")?,
            context,
        )
        .map(Value::Bytes)
    }
}
