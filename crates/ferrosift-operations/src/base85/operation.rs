use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

const STANDARD_ALPHABET: &str = "!-u";

/// Encodes bytes as Base85 (Ascii85) text using an explicit alphabet.
pub struct ToBase85 {
    spec: OperationSpec,
}

impl ToBase85 {
    /// Creates the Base85 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base85.encode@1",
                display_name: "To Base85",
                category: "Encoding",
                description: "Encodes bytes as Base85 text with an explicit alphabet.",
                cyberchef_alias: Some("To Base85"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base85 alphabet range expression.",
                        STANDARD_ALPHABET,
                    ),
                    boolean_argument(
                        "include_delimiter",
                        "Wrap the output in <~ and ~> markers.",
                        false,
                    ),
                ],
                inverse: Some("encoding.base85.decode@1"),
            }),
        }
    }
}

impl Default for ToBase85 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase85 {
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
        let alphabet = codec::parse_alphabet(text_value(arguments, "alphabet")?)?;
        // The reference returns an empty string before the delimiter wrap,
        // so empty input never produces `<~~>`.
        let output = if input.is_empty() {
            String::new()
        } else {
            codec::encode(
                &input,
                &alphabet,
                boolean_value(arguments, "include_delimiter")?,
                context,
            )?
        };
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base85 (Ascii85) text into bytes using an explicit alphabet.
pub struct FromBase85 {
    spec: OperationSpec,
}

impl FromBase85 {
    /// Creates the Base85 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base85.decode@1",
                display_name: "From Base85",
                category: "Encoding",
                description: "Decodes validated Base85 text into bytes.",
                cyberchef_alias: Some("From Base85"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base85 alphabet range expression.",
                        STANDARD_ALPHABET,
                    ),
                    boolean_argument(
                        "remove_non_alphabet",
                        "Remove characters outside the selected alphabet.",
                        true,
                    ),
                    text_argument(
                        "zero_group_character",
                        "Symbol that expands to four zero bytes.",
                        "z",
                    ),
                ],
                inverse: Some("encoding.base85.encode@1"),
            }),
        }
    }
}

impl Default for FromBase85 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase85 {
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
        codec::decode(
            &input.text,
            &alphabet,
            boolean_value(arguments, "remove_non_alphabet")?,
            text_value(arguments, "zero_group_character")?,
            context,
        )
        .map(Value::Bytes)
    }
}
