use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    map_argument, map_value, text_argument, text_value, toggle_string_default, toggle_string_parts,
};
use crate::key::{XOR_INVALID_KEY, convert_to_byte_array};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// HMAC over the input with a selectable hash function.
pub struct Hmac {
    spec: OperationSpec,
}

impl Hmac {
    /// Creates the HMAC operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "hash.hmac@1",
                display_name: "HMAC",
                category: "Hashing",
                description: "Computes an HMAC digest as lower-case hex.",
                cyberchef_alias: Some("HMAC"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    map_argument(
                        "key",
                        "HMAC key as a CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    text_argument(
                        "hashing_function",
                        "Hash function (MD5, SHA1, SHA224, SHA256, SHA384, SHA512, SHA512/224, SHA512/256).",
                        "SHA256",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Hmac {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Hmac {
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
        let key = convert_to_byte_array(string, option, XOR_INVALID_KEY)?;
        let digest = codec::hmac(
            &input,
            &key,
            text_value(arguments, "hashing_function")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: digest,
            encoding: TextEncoding::Utf8,
        }))
    }
}
