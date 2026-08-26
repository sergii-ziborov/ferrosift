use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::jscompat::escape::parse_escaped_chars;
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Marks the characters that every sample shares at the same offset.
pub struct OffsetChecker {
    spec: OperationSpec,
}

impl OffsetChecker {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "analysis.offset_checker@1",
                display_name: "Offset checker",
                category: "Analysis",
                description: "Highlights characters shared by every sample at the same offset.",
                cyberchef_alias: Some("Offset checker"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Markup),
                arguments: vec![text_argument(
                    "sample_delimiter",
                    "Separator between samples, with backslash escapes.",
                    "\\n\\n",
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for OffsetChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for OffsetChecker {
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
        let delimiter = parse_escaped_chars(text_value(arguments, "sample_delimiter")?);
        Ok(Value::Markup(codec::check(
            &input.text,
            &delimiter,
            context,
        )?))
    }
}
