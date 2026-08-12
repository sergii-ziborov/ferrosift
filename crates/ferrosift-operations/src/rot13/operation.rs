use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, integer_argument, integer_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// ROT13 / Caesar rotation over bytes.
pub struct Rot13 {
    spec: OperationSpec,
}

impl Rot13 {
    /// Creates the ROT13 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.rot13@1",
                display_name: "ROT13",
                category: "Encoding",
                description: "Rotates alphabet characters by the specified amount.",
                cyberchef_alias: Some("ROT13"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    boolean_argument("rotate_lower_case_chars", "Rotate a-z.", true),
                    boolean_argument("rotate_upper_case_chars", "Rotate A-Z.", true),
                    boolean_argument("rotate_numbers", "Rotate 0-9.", false),
                    integer_argument("amount", "Rotation amount.", 13),
                ],
                inverse: Some("encoding.rot13@1"),
            }),
        }
    }
}

impl Default for Rot13 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Rot13 {
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
        Ok(Value::Bytes(codec::rot13(
            &input,
            boolean_value(arguments, "rotate_lower_case_chars")?,
            boolean_value(arguments, "rotate_upper_case_chars")?,
            boolean_value(arguments, "rotate_numbers")?,
            integer_value(arguments, "amount")?,
            context,
        )?))
    }
}
