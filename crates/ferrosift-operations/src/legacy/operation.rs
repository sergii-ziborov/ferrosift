use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, integer_argument, integer_value};
use crate::spec::{SpecDefinition, build_reducer};

use super::codec;

/// `MurmurHash3`, reported as a decimal number.
pub struct MurmurHash3 {
    spec: OperationSpec,
}

impl MurmurHash3 {
    /// Creates the `MurmurHash3` operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.murmur3@1",
                display_name: "MurmurHash3",
                category: "Hashing",
                description: "Computes a 32-bit MurmurHash v3 as a decimal number.",
                cyberchef_alias: Some("MurmurHash3"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("seed", "Starting seed.", 0),
                    boolean_argument("signed", "Report as a signed 32-bit integer.", false),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for MurmurHash3 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for MurmurHash3 {
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
        let seed = integer_value(arguments, "seed")?;
        let signed = boolean_value(arguments, "signed")?;
        let input = crate::value::take_text_value(input)?;
        Ok(Value::Text(TextValue {
            text: codec::murmur3(&input.text, seed, signed, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// SHA-0, the withdrawn original.
pub struct Sha0 {
    spec: OperationSpec,
}

impl Sha0 {
    /// Creates the SHA-0 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_reducer(SpecDefinition {
                id: "hash.sha0@1",
                display_name: "SHA0",
                category: "Hashing",
                description: "Computes a SHA-0 digest as lower-case hex.",
                cyberchef_alias: Some("SHA0"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![integer_argument(
                    "rounds",
                    "Compression rounds (only the full 80 is supported).",
                    80,
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Sha0 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Sha0 {
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
        let rounds = integer_value(arguments, "rounds")?;
        let input = crate::value::take_bytes(input)?;
        Ok(Value::Text(TextValue {
            text: codec::sha0(&input, rounds, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
