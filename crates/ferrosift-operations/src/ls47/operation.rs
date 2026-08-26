use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encrypts with LS47.
pub struct Ls47Encrypt {
    spec: OperationSpec,
}

impl Ls47Encrypt {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.ls47.encrypt@1",
                display_name: "LS47 Encrypt",
                category: "Ciphers",
                description: "Encrypts text with the LS47 hand cipher.",
                cyberchef_alias: Some("LS47 Encrypt"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("password", "Password the key is derived from.", ""),
                    integer_argument("padding", "Number of padding characters to prepend.", 10),
                    text_argument("signature", "Text appended after a `---` separator.", ""),
                ],
                inverse: Some("crypto.ls47.decrypt@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for Ls47Encrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ls47Encrypt {
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
        let key = codec::derive_key(text_value(arguments, "password")?)?;
        let padding = integer_value(arguments, "padding")?;
        let signature = text_value(arguments, "signature")?;

        // The reference fills the padding from `Math.random`, so its output is
        // reproducible only when there is none. A non-zero count is refused
        // rather than filled from a generator of this crate's choosing: that
        // would produce a different ciphertext for the same inputs and could
        // never be checked against anything.
        if padding > 0 {
            return Err(failed("crypto.ls47.random_padding_unsupported"));
        }

        let text = codec::encrypt_padded(&key, &input.text, signature, "", context)?;
        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decrypts with LS47.
pub struct Ls47Decrypt {
    spec: OperationSpec,
}

impl Ls47Decrypt {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.ls47.decrypt@1",
                display_name: "LS47 Decrypt",
                category: "Ciphers",
                description: "Decrypts text encrypted with the LS47 hand cipher.",
                cyberchef_alias: Some("LS47 Decrypt"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("password", "Password the key is derived from.", ""),
                    integer_argument("padding", "Number of leading characters to drop.", 10),
                ],
                inverse: Some("crypto.ls47.encrypt@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for Ls47Decrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ls47Decrypt {
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
        let key = codec::derive_key(text_value(arguments, "password")?)?;
        let padding = integer_value(arguments, "padding")?;
        let padding =
            i64::try_from(padding).unwrap_or(if padding < 0 { i64::MIN } else { i64::MAX });
        let text: String = codec::decrypt_padded(&key, &input.text, padding, context)?;
        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}
