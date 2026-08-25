use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Explains a UNIX permission string given in octal or textual form.
pub struct ParseUnixFilePermissions {
    spec: OperationSpec,
}

impl ParseUnixFilePermissions {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "filesystem.unix_permissions@1",
                display_name: "Parse UNIX file permissions",
                category: "Parsing",
                description: "Explains which permissions a UNIX mode grants to which users.",
                cyberchef_alias: Some("Parse UNIX file permissions"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: alloc::vec::Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ParseUnixFilePermissions {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ParseUnixFilePermissions {
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
            text: codec::parse_permissions(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
