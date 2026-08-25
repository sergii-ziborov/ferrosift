use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec::{self, Convert};

/// Renders bytes in the SNORT hex-content notation.
pub struct ToHexContent {
    spec: OperationSpec,
}

impl ToHexContent {
    /// Creates the hex-content encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.hex_content.encode@1",
                display_name: "To Hex Content",
                category: "Encoding",
                description: "Renders non-alphanumeric bytes as pipe-delimited hex.",
                cyberchef_alias: Some("To Hex Content"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "convert",
                        "Which bytes to convert: \"Only special chars\", \
                         \"Only special chars including spaces\", or \"All chars\".",
                        "Only special chars",
                    ),
                    boolean_argument("spaces", "Print spaces between hex bytes.", false),
                ],
                inverse: Some("encoding.hex_content.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToHexContent {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToHexContent {
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
        let convert = Convert::parse(text_value(arguments, "convert")?)?;
        let spaces = boolean_value(arguments, "spaces")?;
        let input = crate::value::take_bytes(input)?;
        Ok(Value::Text(TextValue {
            text: codec::encode(&input, convert, spaces, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Reads the SNORT hex-content notation back into bytes.
pub struct FromHexContent {
    spec: OperationSpec,
}

impl FromHexContent {
    /// Creates the hex-content decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.hex_content.decode@1",
                display_name: "From Hex Content",
                category: "Encoding",
                description: "Reads pipe-delimited hex runs back into bytes.",
                cyberchef_alias: Some("From Hex Content"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("encoding.hex_content.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromHexContent {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromHexContent {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = crate::value::take_text_value(input)?;
        codec::decode(&input.text, context).map(Value::Bytes)
    }
}
