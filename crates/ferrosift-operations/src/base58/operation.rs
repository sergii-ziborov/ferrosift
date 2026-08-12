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

const BITCOIN_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Encodes bytes as Base58 text using an explicit alphabet.
pub struct ToBase58 {
    spec: OperationSpec,
}

impl ToBase58 {
    /// Creates the Base58 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base58.encode@1",
                display_name: "To Base58",
                category: "Encoding",
                description: "Encodes bytes as Base58 text with an explicit alphabet.",
                cyberchef_alias: Some("To Base58"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "alphabet",
                    "Base58 alphabet range expression.",
                    BITCOIN_ALPHABET,
                )],
                inverse: Some("encoding.base58.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBase58 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase58 {
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
        // Unlike Base32, the reference validates the alphabet before the
        // empty-input early return; preserve that observable order.
        let alphabet = codec::parse_alphabet(text_value(arguments, "alphabet")?)?;
        let output = if input.is_empty() {
            String::new()
        } else {
            codec::encode(&input, &alphabet, context)?
        };
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base58 text into bytes using an explicit alphabet.
pub struct FromBase58 {
    spec: OperationSpec,
}

impl FromBase58 {
    /// Creates the Base58 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base58.decode@1",
                display_name: "From Base58",
                category: "Encoding",
                description: "Decodes validated Base58 text into bytes.",
                cyberchef_alias: Some("From Base58"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base58 alphabet range expression.",
                        BITCOIN_ALPHABET,
                    ),
                    boolean_argument(
                        "remove_non_alphabet",
                        "Remove characters outside the selected alphabet.",
                        true,
                    ),
                ],
                inverse: Some("encoding.base58.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBase58 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase58 {
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
        let alphabet = codec::parse_alphabet(text_value(arguments, "alphabet")?)?;
        if input.text.is_empty() {
            return Ok(Value::Bytes(Vec::new()));
        }
        codec::decode(
            &input.text,
            &alphabet,
            boolean_value(arguments, "remove_non_alphabet")?,
            context,
        )
        .map(Value::Bytes)
    }
}
