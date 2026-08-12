use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec::{self, Alphabet};

const STANDARD_ALPHABET: &str = "A-Z2-7=";

/// Encodes bytes as Base32 text using an explicit alphabet.
pub struct ToBase32 {
    spec: OperationSpec,
}

impl ToBase32 {
    /// Creates the Base32 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base32.encode@1",
                display_name: "To Base32",
                category: "Encoding",
                description: "Encodes bytes as Base32 text with an explicit alphabet.",
                cyberchef_alias: Some("To Base32"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "alphabet",
                    "Base32 alphabet range expression.",
                    STANDARD_ALPHABET,
                )],
                inverse: Some("encoding.base32.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBase32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase32 {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        // The reference returns early on empty input, before it ever
        // validates the alphabet; preserve that observable order.
        let output = if input.is_empty() {
            alloc::string::String::new()
        } else {
            let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
            codec::encode(&input, &alphabet, context)?
        };
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base32 text into bytes using an explicit alphabet.
pub struct FromBase32 {
    spec: OperationSpec,
}

impl FromBase32 {
    /// Creates the Base32 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base32.decode@1",
                display_name: "From Base32",
                category: "Encoding",
                description: "Decodes validated Base32 text into bytes.",
                cyberchef_alias: Some("From Base32"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base32 alphabet range expression.",
                        STANDARD_ALPHABET,
                    ),
                    boolean_argument(
                        "remove_non_alphabet",
                        "Remove characters outside the selected alphabet.",
                        true,
                    ),
                ],
                inverse: Some("encoding.base32.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBase32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase32 {
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
        let Value::Text(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        if input.text.is_empty() {
            return Ok(Value::Bytes(Vec::new()));
        }
        let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
        codec::decode(
            &input.text,
            &alphabet,
            boolean_value(arguments, "remove_non_alphabet")?,
            context,
        )
        .map(Value::Bytes)
    }
}
