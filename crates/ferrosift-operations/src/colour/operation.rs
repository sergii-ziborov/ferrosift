use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Reads one colour notation and reports it in all of them.
pub struct ParseColourCode {
    spec: OperationSpec,
}

impl ParseColourCode {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "parsing.colour_code@1",
                display_name: "Parse colour code",
                category: "Parsing",
                description: "Converts a colour between hex, RGB, HSL, and CMYK notations.",
                cyberchef_alias: Some("Parse colour code"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ParseColourCode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ParseColourCode {
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
        Ok(Value::Text(TextValue {
            text: codec::parse(&input.text),
            encoding: TextEncoding::Utf8,
        }))
    }
}