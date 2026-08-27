//! Operation wrappers for the additional digests.

use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    integer_argument, integer_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_bytes, text};

use super::codec::{self, Simple};

fn digest_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Hashing",
        description,
        cyberchef_alias: Some(display_name),
        input: ValueConstraint::Exact(ValueKind::Bytes),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// A digest with a fixed parameter set, reported as lower-case hex.
pub struct FixedDigest {
    spec: OperationSpec,
    which: Simple,
    rounds_argument: Option<&'static str>,
    variant_argument: Option<&'static str>,
}

impl FixedDigest {
    /// MD2.
    #[must_use]
    pub fn md2() -> Self {
        Self {
            spec: digest_spec(
                "hash.md2@1",
                "MD2",
                "Computes an MD2 digest as lower-case hex.",
                vec![integer_argument("rounds", "Compression rounds.", 18)],
            ),
            which: Simple::Md2,
            rounds_argument: Some("rounds"),
            variant_argument: None,
        }
    }

    /// MD4.
    #[must_use]
    pub fn md4() -> Self {
        Self {
            spec: digest_spec(
                "hash.md4@1",
                "MD4",
                "Computes an MD4 digest as lower-case hex.",
                vec![],
            ),
            which: Simple::Md4,
            rounds_argument: None,
            variant_argument: None,
        }
    }

    /// SM3.
    #[must_use]
    pub fn sm3() -> Self {
        Self {
            spec: digest_spec(
                "hash.sm3@1",
                "SM3",
                "Computes an SM3 digest as lower-case hex.",
                vec![
                    integer_argument("length", "Digest length in bits.", 256),
                    integer_argument("rounds", "Compression rounds.", 64),
                ],
            ),
            which: Simple::Sm3,
            rounds_argument: Some("rounds"),
            variant_argument: None,
        }
    }

    /// Whirlpool.
    #[must_use]
    pub fn whirlpool() -> Self {
        Self {
            spec: digest_spec(
                "hash.whirlpool@1",
                "Whirlpool",
                "Computes a Whirlpool digest as lower-case hex.",
                vec![
                    text_argument("variant", "Whirlpool variant.", "Whirlpool"),
                    integer_argument("rounds", "Compression rounds.", 10),
                ],
            ),
            which: Simple::Whirlpool,
            rounds_argument: Some("rounds"),
            variant_argument: Some("variant"),
        }
    }
}

impl Operation for FixedDigest {
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
        if let Some(name) = self.variant_argument {
            codec::check_whirlpool_variant(text_value(arguments, name)?)?;
        }
        let rounds = match self.rounds_argument {
            Some(name) => Some(integer_value(arguments, name)?),
            None => None,
        };
        // SM3 offers a length too, and 256 is the only published one.
        if matches!(self.which, Simple::Sm3) && integer_value(arguments, "length")? != 256 {
            return Err(OperationError::InvalidArguments);
        }
        let input = take_bytes(input)?;
        Ok(text(codec::simple(&input, self.which, rounds, context)?))
    }
}

/// RIPEMD at one of its four published sizes.
pub struct Ripemd {
    spec: OperationSpec,
}

impl Ripemd {
    /// Creates the RIPEMD operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: digest_spec(
                "hash.ripemd@1",
                "RIPEMD",
                "Computes a RIPEMD digest as lower-case hex.",
                vec![text_argument(
                    "size",
                    "Digest size: 128, 160, 256, or 320.",
                    "320",
                )],
            ),
        }
    }
}

impl Default for Ripemd {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ripemd {
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
        let size: i128 = text_value(arguments, "size")?
            .parse()
            .map_err(|_| OperationError::InvalidArguments)?;
        let input = take_bytes(input)?;
        Ok(text(codec::ripemd(&input, size, context)?))
    }
}

/// Streebog, the Russian standard hash, at one of its two digest lengths.
pub struct Streebog {
    spec: OperationSpec,
}

impl Streebog {
    /// Creates the Streebog operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: digest_spec(
                "hash.streebog@1",
                "Streebog",
                "Computes a Streebog (GOST R 34.11-2012) digest as lower-case hex.",
                vec![text_argument(
                    "size",
                    "Digest length in bits: 256 or 512.",
                    "256",
                )],
            ),
        }
    }
}

impl Default for Streebog {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Streebog {
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
        let size: i128 = text_value(arguments, "size")?
            .parse()
            .map_err(|_| OperationError::InvalidArguments)?;
        let input = take_bytes(input)?;
        Ok(text(codec::streebog(&input, size, context)?))
    }
}

/// BLAKE2, in its sixty-four and thirty-two bit forms.
///
/// One struct for two operations, which differ only in which function they
/// name and which digest sizes that function can produce.
pub struct Blake2 {
    spec: OperationSpec,
    kind: codec::Blake2Kind,
}

impl Blake2 {
    fn build(
        id: &'static str,
        display_name: &'static str,
        sizes: &'static str,
        default_size: &'static str,
        kind: codec::Blake2Kind,
    ) -> Self {
        Self {
            spec: build(SpecDefinition {
                id,
                display_name,
                category: "Hashing",
                description: "Computes a BLAKE2 digest, optionally keyed.",
                cyberchef_alias: Some(display_name),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("size", sizes, default_size),
                    text_argument(
                        "output_encoding",
                        "How to write the digest: Hex, Base64, or Raw.",
                        "Hex",
                    ),
                    map_argument(
                        "key",
                        "Key as CyberChef toggleString; empty means unkeyed.",
                        toggle_string_default("UTF8", ""),
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
            kind,
        }
    }

    /// `BLAKE2b`.
    #[must_use]
    pub fn b() -> Self {
        Self::build(
            "hash.blake2b@1",
            "BLAKE2b",
            "Digest length in bits: 512, 384, 256, 160, or 128.",
            "512",
            codec::Blake2Kind::B,
        )
    }

    /// `BLAKE2s`.
    #[must_use]
    pub fn s() -> Self {
        Self::build(
            "hash.blake2s@1",
            "BLAKE2s",
            "Digest length in bits: 256, 160, or 128.",
            "256",
            codec::Blake2Kind::S,
        )
    }
}

impl Operation for Blake2 {
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
        let size: i128 = text_value(arguments, "size")?
            .parse()
            .map_err(|_| OperationError::InvalidArguments)?;
        let encoding = text_value(arguments, "output_encoding")?;
        let (key_option, key_text) = toggle_string_parts(map_value(arguments, "key")?)?;
        let key =
            crate::key::convert_to_byte_array(key_text, key_option, "hash.blake2.invalid_key")?;
        // The reference refuses a key longer than the function allows, and
        // says so rather than truncating it. The two functions do not allow
        // the same length: sixty-four bytes for the sixty-four bit form and
        // thirty-two for the other, which is half of each one's block.
        let limit = match self.kind {
            codec::Blake2Kind::B => 64,
            codec::Blake2Kind::S => 32,
        };
        if key.len() > limit {
            return Err(crate::failure::failed("hash.blake2.key_too_long"));
        }
        let input = take_bytes(input)?;

        let digest = codec::blake2(&input, self.kind, size, &key, context)?;
        context.ensure_active()?;
        // Raw is the bytes themselves, carried as the code points the
        // reference's dish would hold -- one per byte, never a UTF-8 encoding
        // of them.
        let written = match encoding {
            "Base64" => crate::base64::encode_standard(&digest, context)?,
            "Raw" => digest.iter().map(|byte| char::from(*byte)).collect(),
            _ => crate::hex_util::to_hex_lower(&digest),
        };
        Ok(text(written))
    }
}
