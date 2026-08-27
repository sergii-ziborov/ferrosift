use alloc::{collections::BTreeSet, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationClassification, OperationSpec, TextEncoding, TextValue, Value,
    ValueConstraint, ValueKind,
};

use crate::args::{
    map_argument, map_value, text_argument, text_value, toggle_string_default, toggle_string_parts,
};
use crate::codec_bytes::{decode_input, encode_output, toggle_bytes};
use crate::spec::{SpecDefinition, build};

use super::codec::{self, DecryptParams, EncryptParams};

fn text_kinds() -> ValueConstraint {
    ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]))
}

/// AES encryption (CBC / CFB / OFB / CTR / ECB / GCM).
pub struct AesEncrypt {
    spec: OperationSpec,
}

impl AesEncrypt {
    /// Creates the AES encrypt operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.aes.encrypt@1",
                display_name: "AES Encrypt",
                category: "Ciphers",
                description: "Encrypts data with AES-CBC, CFB, OFB, CTR, ECB, or GCM.",
                cyberchef_alias: Some("AES Encrypt"),
                input: text_kinds(),
                output: text_kinds(),
                arguments: vec![
                    map_argument(
                        "key",
                        "AES key as CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    map_argument(
                        "iv",
                        "Initialization vector; empty defaults to 16 null bytes.",
                        toggle_string_default("Hex", ""),
                    ),
                    text_argument(
                        "mode",
                        "CBC, CFB, OFB, CTR, ECB, GCM, CBC/NoPadding, or ECB/NoPadding.",
                        "CBC",
                    ),
                    text_argument("input", "Input encoding: Raw or Hex.", "Raw"),
                    text_argument("output", "Output encoding: Hex or Raw.", "Hex"),
                    map_argument(
                        "additional_authenticated_data",
                        "GCM AAD as toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    text_argument("include_iv_in_output", "Off, Prepend, or Append.", "Off"),
                ],
                inverse: Some("crypto.aes.decrypt@1"),
                classifications: Some(&[OperationClassification::Unsafe]),
            }),
        }
    }
}

impl Default for AesEncrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AesEncrypt {
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
        let (aad_opt, aad_str) =
            toggle_string_parts(map_value(arguments, "additional_authenticated_data")?)?;
        let key = toggle_bytes(key_opt, key_str);
        let iv = toggle_bytes(iv_opt, iv_str);
        let aad = toggle_bytes(aad_opt, aad_str);
        let mode = text_value(arguments, "mode")?;
        let input_format = text_value(arguments, "input")?;
        let output_format = text_value(arguments, "output")?;
        let include_iv = text_value(arguments, "include_iv_in_output")?;
        let plaintext = decode_input(input, input_format)?;
        let (body, tag) = codec::encrypt(
            &plaintext,
            &EncryptParams {
                key: &key,
                iv: &iv,
                mode,
                aad: &aad,
                include_iv,
            },
            context,
        )?;
        if output_format == "Hex" || tag.is_some() {
            let text = codec::format_encrypt_output(&body, tag.as_deref(), output_format)?;
            if output_format == "Hex" {
                Ok(Value::Text(TextValue {
                    text,
                    encoding: TextEncoding::Utf8,
                }))
            } else {
                // Raw + GCM tag annotation as latin1 text container.
                Ok(Value::Text(TextValue {
                    text,
                    encoding: TextEncoding::Utf8,
                }))
            }
        } else {
            encode_output(&body, output_format)
        }
    }
}

/// AES decryption (CBC / CFB / OFB / CTR / ECB / GCM).
pub struct AesDecrypt {
    spec: OperationSpec,
}

impl AesDecrypt {
    /// Creates the AES decrypt operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.aes.decrypt@1",
                display_name: "AES Decrypt",
                category: "Ciphers",
                description: "Decrypts data with AES-CBC, CFB, OFB, CTR, ECB, or GCM.",
                cyberchef_alias: Some("AES Decrypt"),
                input: text_kinds(),
                output: text_kinds(),
                arguments: vec![
                    map_argument(
                        "key",
                        "AES key as CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    map_argument(
                        "iv",
                        "Initialization vector; empty defaults to 16 null bytes.",
                        toggle_string_default("Hex", ""),
                    ),
                    crate::args::integer_argument(
                        "iv_length",
                        "IV length when extracting IV from the input.",
                        16,
                    ),
                    text_argument(
                        "mode",
                        "CBC, CFB, OFB, CTR, ECB, GCM, CBC/NoPadding, or ECB/NoPadding.",
                        "CBC",
                    ),
                    text_argument("input", "Input encoding: Hex or Raw.", "Hex"),
                    text_argument("output", "Output encoding: Raw or Hex.", "Raw"),
                    map_argument(
                        "gcm_tag",
                        "GCM authentication tag as toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    map_argument(
                        "additional_authenticated_data",
                        "GCM AAD as toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    text_argument("iv_from_input", "Off, From start, or From end.", "Off"),
                ],
                inverse: Some("crypto.aes.encrypt@1"),
                classifications: Some(&[OperationClassification::Unsafe]),
            }),
        }
    }
}

impl Default for AesDecrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AesDecrypt {
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
        let (tag_opt, tag_str) = toggle_string_parts(map_value(arguments, "gcm_tag")?)?;
        let (aad_opt, aad_str) =
            toggle_string_parts(map_value(arguments, "additional_authenticated_data")?)?;
        let key = toggle_bytes(key_opt, key_str);
        let mut iv = toggle_bytes(iv_opt, iv_str);
        let tag = toggle_bytes(tag_opt, tag_str);
        let aad = toggle_bytes(aad_opt, aad_str);
        let mode = text_value(arguments, "mode")?;
        let input_format = text_value(arguments, "input")?;
        let output_format = text_value(arguments, "output")?;
        let iv_from_input = text_value(arguments, "iv_from_input")?;
        let iv_length = crate::args::integer_value(arguments, "iv_length")?;
        let mut ciphertext = decode_input(input, input_format)?;
        if iv_from_input != "Off" {
            let len = usize::try_from(iv_length).map_err(|_| OperationError::InvalidArguments)?;
            if ciphertext.len() <= len {
                return Err(OperationError::InvalidArguments);
            }
            if iv_from_input == "From start" {
                iv = ciphertext[..len].to_vec();
                ciphertext = ciphertext[len..].to_vec();
            } else if iv_from_input == "From end" {
                let split = ciphertext.len() - len;
                iv = ciphertext[split..].to_vec();
                ciphertext.truncate(split);
            } else {
                return Err(OperationError::InvalidArguments);
            }
        }
        let plain = codec::decrypt(
            &ciphertext,
            &DecryptParams {
                key: &key,
                iv: &iv,
                mode,
                tag: &tag,
                aad: &aad,
            },
            context,
        )?;
        encode_output(&plain, output_format)
    }
}
