use alloc::{collections::BTreeSet, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationClassification, OperationSpec, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    map_argument, map_value, text_argument, text_value, toggle_string_default, toggle_string_parts,
};
use crate::codec_bytes::{decode_input, encode_output, toggle_bytes};
use crate::spec::{SpecDefinition, build};

use super::codec;

fn text_kinds() -> ValueConstraint {
    ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]))
}

/// AES Key Wrap (RFC 3394).
pub struct AesKeyWrap {
    spec: OperationSpec,
}

impl AesKeyWrap {
    /// Creates the AES Key Wrap operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.aes_kw.wrap@1",
                display_name: "AES Key Wrap",
                category: "Ciphers",
                description: "Wraps a key with AES Key Wrap (RFC 3394).",
                cyberchef_alias: Some("AES Key Wrap"),
                input: text_kinds(),
                output: text_kinds(),
                arguments: vec![
                    map_argument(
                        "key",
                        "Key-encryption key (KEK) as CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    map_argument(
                        "iv",
                        "64-bit integrity check value; default RFC 3394 IV.",
                        toggle_string_default("Hex", "a6a6a6a6a6a6a6a6"),
                    ),
                    text_argument("input", "Input encoding: Hex or Raw.", "Hex"),
                    text_argument("output", "Output encoding: Hex or Raw.", "Hex"),
                ],
                inverse: Some("crypto.aes_kw.unwrap@1"),
                classifications: Some(&[OperationClassification::Unsafe]),
            }),
        }
    }
}

impl Default for AesKeyWrap {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AesKeyWrap {
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
        let (key_opt, key_str) = toggle_string_parts(map_value(arguments, "key")?)?;
        let (iv_opt, iv_str) = toggle_string_parts(map_value(arguments, "iv")?)?;
        let kek = toggle_bytes(key_opt, key_str)?;
        let iv = toggle_bytes(iv_opt, iv_str)?;
        let input_format = text_value(arguments, "input")?;
        let output_format = text_value(arguments, "output")?;
        let plaintext = decode_input(input, input_format)?;
        let wrapped = codec::wrap(&plaintext, &kek, &iv, context)?;
        encode_output(&wrapped, output_format)
    }
}

/// AES Key Unwrap (RFC 3394).
pub struct AesKeyUnwrap {
    spec: OperationSpec,
}

impl AesKeyUnwrap {
    /// Creates the AES Key Unwrap operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.aes_kw.unwrap@1",
                display_name: "AES Key Unwrap",
                category: "Ciphers",
                description: "Unwraps a key with AES Key Wrap (RFC 3394).",
                cyberchef_alias: Some("AES Key Unwrap"),
                input: text_kinds(),
                output: text_kinds(),
                arguments: vec![
                    map_argument(
                        "key",
                        "Key-encryption key (KEK) as CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    map_argument(
                        "iv",
                        "64-bit integrity check value; default RFC 3394 IV.",
                        toggle_string_default("Hex", "a6a6a6a6a6a6a6a6"),
                    ),
                    text_argument("input", "Input encoding: Hex or Raw.", "Hex"),
                    text_argument("output", "Output encoding: Hex or Raw.", "Hex"),
                ],
                inverse: Some("crypto.aes_kw.wrap@1"),
                classifications: Some(&[OperationClassification::Unsafe]),
            }),
        }
    }
}

impl Default for AesKeyUnwrap {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AesKeyUnwrap {
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
        let (key_opt, key_str) = toggle_string_parts(map_value(arguments, "key")?)?;
        let (iv_opt, iv_str) = toggle_string_parts(map_value(arguments, "iv")?)?;
        let kek = toggle_bytes(key_opt, key_str)?;
        let iv = toggle_bytes(iv_opt, iv_str)?;
        let input_format = text_value(arguments, "input")?;
        let output_format = text_value(arguments, "output")?;
        let ciphertext = decode_input(input, input_format)?;
        let plain = codec::unwrap(&ciphertext, &kek, &iv, context)?;
        encode_output(&plain, output_format)
    }
}
