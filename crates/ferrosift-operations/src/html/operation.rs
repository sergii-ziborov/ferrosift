use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encodes characters as HTML entities.
pub struct ToHtmlEntity {
    spec: OperationSpec,
}

impl ToHtmlEntity {
    /// Creates the HTML entity encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.html.encode@1",
                display_name: "To HTML Entity",
                category: "Encoding",
                description: "Converts characters to HTML entities.",
                cyberchef_alias: Some("To HTML Entity"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    boolean_argument(
                        "convert_all_characters",
                        "Encode every character, not only specials.",
                        false,
                    ),
                    text_argument(
                        "convert_to",
                        "Named entities, Numeric entities, or Hex entities.",
                        "Named entities",
                    ),
                ],
                inverse: Some("encoding.html.decode@1"),
            }),
        }
    }
}

impl Default for ToHtmlEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToHtmlEntity {
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
                boolean_value(arguments, "convert_all_characters")?,
                text_value(arguments, "convert_to")?,
                context,
            )?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes HTML entities back to characters.
pub struct FromHtmlEntity {
    spec: OperationSpec,
}

impl FromHtmlEntity {
    /// Creates the HTML entity decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.html.decode@1",
                display_name: "From HTML Entity",
                category: "Encoding",
                description: "Converts HTML entities back to characters.",
                cyberchef_alias: Some("From HTML Entity"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: Some("encoding.html.encode@1"),
            }),
        }
    }
}

impl Default for FromHtmlEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromHtmlEntity {
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
        Ok(Value::Text(TextValue {
            text: codec::decode(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
