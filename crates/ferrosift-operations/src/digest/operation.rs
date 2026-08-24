//! Operation wrappers for the additional digests.

use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
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
