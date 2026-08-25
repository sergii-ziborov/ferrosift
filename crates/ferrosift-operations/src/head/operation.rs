use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Keeps the first n delimited fields, like UNIX `head`.
pub struct Head {
    spec: OperationSpec,
}

impl Head {
    /// Creates the head operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "data.head@1",
                display_name: "Head",
                category: "Data",
                description: "Gets the first n delimited fields from the input.",
                cyberchef_alias: Some("Head"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Field delimiter token.", "Line feed"),
                    integer_argument(
                        "number",
                        "Number of fields to keep; negative drops the last -n.",
                        10,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Head {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Head {
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
        let output = codec::head(
            &input.text,
            text_value(arguments, "delimiter")?,
            integer_value(arguments, "number")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}
