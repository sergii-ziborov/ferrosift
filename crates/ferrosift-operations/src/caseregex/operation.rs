use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Rewrites a regular expression to match either case without the `i` flag.
pub struct ToCaseInsensitiveRegex {
    spec: OperationSpec,
}

impl ToCaseInsensitiveRegex {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "text.regex.case_widen@1",
                display_name: "To Case Insensitive Regex",
                category: "Text",
                description: "Rewrites a regex so it matches either case without the i flag.",
                cyberchef_alias: Some("To Case Insensitive Regex"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: alloc::vec::Vec::new(),
                inverse: Some("text.regex.case_fold@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToCaseInsensitiveRegex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToCaseInsensitiveRegex {
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
            text: codec::to_case_insensitive(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
