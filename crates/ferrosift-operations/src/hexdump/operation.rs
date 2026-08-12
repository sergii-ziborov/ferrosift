use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, integer_argument, integer_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Creates a classic hexdump of the input bytes.
pub struct ToHexdump {
    spec: OperationSpec,
}

impl ToHexdump {
    /// Creates the hexdump encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.hexdump.encode@1",
                display_name: "To Hexdump",
                category: "Encoding",
                description: "Creates a hexdump of the input data.",
                cyberchef_alias: Some("To Hexdump"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("width", "Bytes per dump line.", 16),
                    boolean_argument(
                        "upper_case_hex",
                        "Emit upper-case hexadecimal digits.",
                        false,
                    ),
                    boolean_argument(
                        "include_final_length",
                        "Append a final line with the total length.",
                        false,
                    ),
                    boolean_argument(
                        "unix_format",
                        "Restrict the ASCII preview to printable ASCII.",
                        false,
                    ),
                ],
                inverse: Some("encoding.hexdump.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToHexdump {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToHexdump {
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
        let output = codec::encode(
            &input,
            integer_value(arguments, "width")?,
            boolean_value(arguments, "upper_case_hex")?,
            boolean_value(arguments, "include_final_length")?,
            boolean_value(arguments, "unix_format")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Parses common hexdump formats back into raw bytes.
pub struct FromHexdump {
    spec: OperationSpec,
}

impl FromHexdump {
    /// Creates the hexdump decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.hexdump.decode@1",
                display_name: "From Hexdump",
                category: "Encoding",
                description: "Converts a hexdump back into raw data.",
                cyberchef_alias: Some("From Hexdump"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("encoding.hexdump.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromHexdump {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromHexdump {
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
        let Value::Text(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::decode(&input.text, context)?))
    }
}
