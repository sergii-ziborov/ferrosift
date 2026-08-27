use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::key::convert_to_byte_array;
use crate::spec::{SpecDefinition, build};

use super::codec;

/// XOR the input with a repeating or differential key.
pub struct Xor {
    spec: OperationSpec,
}

impl Xor {
    /// Creates the XOR operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "logic.xor@1",
                display_name: "XOR",
                category: "Logic",
                description: "XOR the input with the given key.",
                cyberchef_alias: Some("XOR"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    map_argument(
                        "key",
                        "Key value and encoding (CyberChef toggleString).",
                        toggle_string_default("Hex", ""),
                    ),
                    text_argument("scheme", "Key update scheme.", "Standard"),
                    boolean_argument(
                        "null_preserving",
                        "Skip XOR when the input byte is 0x00 or equals the key byte.",
                        false,
                    ),
                ],
                inverse: Some("logic.xor@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for Xor {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Xor {
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
        let input = crate::value::take_bytes(input)?;
        let (option, string) = toggle_string_parts(map_value(arguments, "key")?)?;
        let key = convert_to_byte_array(string, option);
        let scheme = text_value(arguments, "scheme")?;
        let null_preserving = boolean_value(arguments, "null_preserving")?;
        let output = codec::apply(&input, &key, scheme, null_preserving, context)?;
        Ok(Value::Bytes(output))
    }
}
