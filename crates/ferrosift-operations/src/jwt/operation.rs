use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Decodes a compact JWT and returns the payload object.
///
/// Does not verify the signature. Use a dedicated verify operation for that.
pub struct JwtDecode {
    spec: OperationSpec,
}

impl JwtDecode {
    /// Creates the JWT decode operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "parsing.jwt.decode@1",
                display_name: "JWT Decode",
                category: "Parsing",
                description: "Decodes a compact JWT and returns the JSON payload without verifying the signature.",
                cyberchef_alias: Some("JWT Decode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Structured),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for JwtDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for JwtDecode {
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
        Ok(Value::Structured(codec::decode(&input.text, context)?))
    }
}
