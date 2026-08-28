use alloc::boxed::Box;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError, StreamSession, Streamable};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build_reducer, incremental};
use crate::stream::DigestSession;

use super::codec;

/// SHA-3 message digest (FIPS 202).
pub struct Sha3 {
    spec: OperationSpec,
}

impl Sha3 {
    /// Creates the SHA-3 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.sha3@1",
                display_name: "SHA3",
                category: "Hashing",
                description: "Computes a SHA-3 digest as lower-case hex.",
                cyberchef_alias: Some("SHA3"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "size",
                    "Digest size: 224, 256, 384, or 512.",
                    "512",
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Sha3 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Sha3 {
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
        Ok(text(codec::sha3(
            &input,
            text_value(arguments, "size")?,
            context,
        )?))
    }
}

/// MD5 message digest (hex lower-case).
pub struct Md5 {
    spec: OperationSpec,
}

impl Md5 {
    /// Creates the MD5 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.md5@1",
                display_name: "MD5",
                category: "Hashing",
                description: "Computes the MD5 digest as lower-case hex.",
                cyberchef_alias: Some("MD5"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Md5 {
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
        let input = crate::value::take_bytes(input)?;
        Ok(text(codec::md5(&input, context)?))
    }
}

/// SHA-1 message digest (full 80 rounds).
pub struct Sha1 {
    spec: OperationSpec,
}

impl Sha1 {
    /// Creates the SHA-1 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.sha1@1",
                display_name: "SHA1",
                category: "Hashing",
                description: "Computes the SHA-1 digest as lower-case hex.",
                cyberchef_alias: Some("SHA1"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![integer_argument(
                    "rounds",
                    "Number of SHA-1 rounds (only the full 80 is supported).",
                    80,
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Sha1 {
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
        Ok(text(codec::sha1(
            &input,
            integer_value(arguments, "rounds")?,
            context,
        )?))
    }
}

/// SHA-2 family digests.
pub struct Sha2 {
    spec: OperationSpec,
}

impl Sha2 {
    /// Creates the SHA-2 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: incremental(build_reducer(SpecDefinition {
                id: "hash.sha2@1",
                display_name: "SHA2",
                category: "Hashing",
                description: "Computes a SHA-2 family digest as lower-case hex.",
                cyberchef_alias: Some("SHA2"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "size",
                        "Digest size: 224, 256, 384, 512, 512/224, 512/256.",
                        "256",
                    ),
                    integer_argument(
                        "rounds_256",
                        "Rounds for SHA-224/256 (only the full 64 is supported).",
                        64,
                    ),
                    integer_argument(
                        "rounds_512",
                        "Rounds for SHA-384/512 (only the full 160 is supported).",
                        160,
                    ),
                ],
                inverse: None,
                classifications: None,
            })),
        }
    }
}

impl Default for Sha2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Sha2 {
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
        Ok(text(codec::sha2(
            &input,
            text_value(arguments, "size")?,
            integer_value(arguments, "rounds_256")?,
            integer_value(arguments, "rounds_512")?,
            context,
        )?))
    }
}

impl Streamable for Sha2 {
    fn start(
        &self,
        arguments: &Arguments,
        _context: &OperationContext<'_>,
    ) -> Result<Option<Box<dyn StreamSession + '_>>, OperationError> {
        // The reduced-round variants are refused by the buffered path too, and
        // for the same reason: they are a different function. `None` here
        // would be a session that silently declined; the error is the answer.
        let digest = codec::sha2_streaming(
            text_value(arguments, "size")?,
            integer_value(arguments, "rounds_256")?,
            integer_value(arguments, "rounds_512")?,
        )?;
        Ok(Some(Box::new(DigestSession::new(digest))))
    }
}

/// Keccak as submitted to the SHA-3 competition, before the padding changed.
pub struct Keccak {
    spec: OperationSpec,
}

impl Keccak {
    /// Creates the Keccak operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.keccak@1",
                display_name: "Keccak",
                category: "Hashing",
                description: "Computes an original Keccak digest as lower-case hex.",
                cyberchef_alias: Some("Keccak"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "size",
                    "Digest size: 224, 256, 384, or 512.",
                    "512",
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Keccak {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Keccak {
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
        Ok(text(codec::keccak(
            &input,
            text_value(arguments, "size")?,
            context,
        )?))
    }
}

/// SHAKE, the extendable-output function of the Keccak family.
pub struct Shake {
    spec: OperationSpec,
}

impl Shake {
    /// Creates the SHAKE operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.shake@1",
                display_name: "Shake",
                category: "Hashing",
                description: "Computes a SHAKE digest of the requested length as lower-case hex.",
                cyberchef_alias: Some("Shake"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("capacity", "Security level: 128 or 256.", "256"),
                    integer_argument("size", "Output size in bits, a multiple of 8.", 512),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Shake {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Shake {
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
        Ok(text(codec::shake(
            &input,
            text_value(arguments, "capacity")?,
            integer_value(arguments, "size")?,
            context,
        )?))
    }
}

fn text(value: alloc::string::String) -> Value {
    Value::Text(TextValue {
        text: value,
        encoding: TextEncoding::Utf8,
    })
}
