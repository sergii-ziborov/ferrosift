use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Converts text to unicode character codes.
pub struct ToCharcode {
    spec: OperationSpec,
}

impl ToCharcode {
    /// Creates the charcode encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.charcode.encode@1",
                display_name: "To Charcode",
                category: "Encoding",
                description: "Converts text to unicode character codes.",
                cyberchef_alias: Some("To Charcode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Delimiter between codes.", "Space"),
                    integer_argument("base", "Numeric base for codes (2..=36).", 16),
                ],
                inverse: Some("encoding.charcode.decode@1"),
            }),
        }
    }
}

impl Default for ToCharcode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToCharcode {
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
        Ok(Value::Text(TextValue {
            text: codec::encode(
                &input.text,
                text_value(arguments, "delimiter")?,
                integer_value(arguments, "base")?,
                context,
            )?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Converts character codes back to bytes.
pub struct FromCharcode {
    spec: OperationSpec,
}

impl FromCharcode {
    /// Creates the charcode decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.charcode.decode@1",
                display_name: "From Charcode",
                category: "Encoding",
                description: "Converts unicode character codes back into bytes.",
                cyberchef_alias: Some("From Charcode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument("delimiter", "Delimiter between codes.", "Space"),
                    integer_argument("base", "Numeric base for codes (2..=36).", 16),
                ],
                inverse: Some("encoding.charcode.encode@1"),
            }),
        }
    }
}

impl Default for FromCharcode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromCharcode {
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
        Ok(Value::Bytes(codec::decode(
            &input.text,
            text_value(arguments, "delimiter")?,
            integer_value(arguments, "base")?,
            context,
        )?))
    }
}
