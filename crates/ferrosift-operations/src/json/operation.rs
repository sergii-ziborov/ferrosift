use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Removes insignificant whitespace from JSON text.
pub struct JsonMinify {
    spec: OperationSpec,
}

impl JsonMinify {
    /// Creates the JSON minify operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "text.json.minify@1",
                display_name: "JSON Minify",
                category: "Text",
                description: "Removes whitespace outside JSON strings.",
                cyberchef_alias: Some("JSON Minify"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for JsonMinify {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for JsonMinify {
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
            text: codec::minify(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
