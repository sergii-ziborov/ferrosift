use alloc::{collections::BTreeSet, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    integer_argument, integer_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::codec_bytes::toggle_bytes;
use crate::spec::{SpecDefinition, build};

use super::codec;

fn text_kinds() -> ValueConstraint {
    ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]))
}

/// Derive PBKDF2 key (fixed salt only; empty salt is rejected).
pub struct DerivePbkdf2Key {
    spec: OperationSpec,
}

impl DerivePbkdf2Key {
    /// Creates the Derive PBKDF2 key operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.pbkdf2@1",
                display_name: "Derive PBKDF2 key",
                category: "Ciphers",
                description: "Derives a key with PBKDF2-HMAC. Empty salt is rejected for determinism.",
                cyberchef_alias: Some("Derive PBKDF2 key"),
                input: text_kinds(),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    map_argument(
                        "passphrase",
                        "Passphrase as CyberChef toggleString.",
                        toggle_string_default("UTF8", ""),
                    ),
                    integer_argument("key_size", "Derived key size in bits.", 128),
                    integer_argument("iterations", "PBKDF2 iteration count.", 1),
                    text_argument(
                        "hashing_function",
                        "HMAC hash: SHA1, SHA256, SHA384, SHA512, or MD5.",
                        "SHA1",
                    ),
                    map_argument(
                        "salt",
                        "Salt as CyberChef toggleString (required; not random).",
                        toggle_string_default("Hex", ""),
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for DerivePbkdf2Key {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DerivePbkdf2Key {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        _input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let (pass_opt, pass_str) = toggle_string_parts(map_value(arguments, "passphrase")?)?;
        let (salt_opt, salt_str) = toggle_string_parts(map_value(arguments, "salt")?)?;
        let passphrase = toggle_bytes(pass_opt, pass_str)?;
        let salt = toggle_bytes(salt_opt, salt_str)?;
        let hex = codec::pbkdf2_key(
            &passphrase,
            integer_value(arguments, "key_size")?,
            integer_value(arguments, "iterations")?,
            text_value(arguments, "hashing_function")?,
            &salt,
            context,
        )?;
        Ok(text(hex))
    }
}

/// Scrypt password-based key derivation.
pub struct Scrypt {
    spec: OperationSpec,
}

impl Scrypt {
    /// Creates the Scrypt operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.scrypt@1",
                display_name: "Scrypt",
                category: "Crypto",
                description: "Derives a key with scrypt (RFC 7914).",
                cyberchef_alias: Some("Scrypt"),
                input: text_kinds(),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    map_argument(
                        "salt",
                        "Salt as CyberChef toggleString.",
                        toggle_string_default("Hex", ""),
                    ),
                    integer_argument(
                        "iterations",
                        "CPU/memory cost parameter N (power of two).",
                        16384,
                    ),
                    integer_argument("memory_factor", "Block size parameter r.", 8),
                    integer_argument("parallelization_factor", "Parallelization parameter p.", 1),
                    integer_argument("key_length", "Derived key length in bytes.", 64),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Scrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Scrypt {
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
        let password = match input {
            Value::Bytes(bytes) => bytes,
            Value::Text(text) => text.text.into_bytes(),
            _ => return Err(OperationError::InvalidArguments),
        };
        let (salt_opt, salt_str) = toggle_string_parts(map_value(arguments, "salt")?)?;
        let salt = toggle_bytes(salt_opt, salt_str)?;
        let hex = codec::scrypt_key(
            &password,
            &salt,
            integer_value(arguments, "iterations")?,
            integer_value(arguments, "memory_factor")?,
            integer_value(arguments, "parallelization_factor")?,
            integer_value(arguments, "key_length")?,
            context,
        )?;
        Ok(text(hex))
    }
}

fn text(value: alloc::string::String) -> Value {
    Value::Text(TextValue {
        text: value,
        encoding: TextEncoding::Utf8,
    })
}
