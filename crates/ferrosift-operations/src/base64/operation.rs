use alloc::{string::String, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    ArgumentKind, ArgumentSpec, ArgumentValue, Arguments, OperationSpec, TextEncoding, TextValue,
    Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::{alphabet::Alphabet, codec};

const STANDARD_ALPHABET: &str = "A-Za-z0-9+/=";

/// Encodes bytes as Base64 text using an explicit alphabet.
pub struct ToBase64 {
    spec: OperationSpec,
}

impl ToBase64 {
    /// Creates the Base64 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base64.encode@1",
                display_name: "To Base64",
                category: "Encoding",
                description: "Encodes bytes as Base64 text with an explicit alphabet.",
                cyberchef_alias: Some("To Base64"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "alphabet",
                    "Base64 alphabet range expression.",
                    STANDARD_ALPHABET,
                )],
                inverse: Some("encoding.base64.decode@1"),
            }),
        }
    }
}

impl Default for ToBase64 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase64 {
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
        let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
        let output = codec::encode(&input, &alphabet, context)?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base64 text into bytes using an explicit alphabet.
pub struct FromBase64 {
    spec: OperationSpec,
}

impl FromBase64 {
    /// Creates the Base64 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base64.decode@1",
                display_name: "From Base64",
                category: "Encoding",
                description: "Decodes validated Base64 text into bytes.",
                cyberchef_alias: Some("From Base64"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "Base64 alphabet range expression.",
                        STANDARD_ALPHABET,
                    ),
                    boolean_argument(
                        "remove_non_alphabet",
                        "Remove characters outside the selected alphabet.",
                        true,
                    ),
                    boolean_argument("strict", "Apply strict shape validation.", false),
                ],
                inverse: Some("encoding.base64.encode@1"),
            }),
        }
    }
}

impl Default for FromBase64 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase64 {
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
        let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
        codec::decode(
            &input.text,
            &alphabet,
            boolean_value(arguments, "remove_non_alphabet")?,
            boolean_value(arguments, "strict")?,
            context,
        )
        .map(Value::Bytes)
    }
}

fn text_argument(name: &str, description: &str, default: &str) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Text,
        default: Some(ArgumentValue::Text(String::from(default))),
    }
}

fn boolean_argument(name: &str, description: &str, default: bool) -> ArgumentSpec {
    ArgumentSpec {
        name: String::from(name),
        description: String::from(description),
        required: false,
        kind: ArgumentKind::Boolean,
        default: Some(ArgumentValue::Boolean(default)),
    }
}

fn text_value<'a>(arguments: &'a Arguments, name: &str) -> Result<&'a str, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Text(value)) => Ok(value),
        _ => Err(OperationError::InvalidArguments),
    }
}

fn boolean_value(arguments: &Arguments, name: &str) -> Result<bool, OperationError> {
    match arguments.get(name) {
        Some(ArgumentValue::Boolean(value)) => Ok(*value),
        _ => Err(OperationError::InvalidArguments),
    }
}
