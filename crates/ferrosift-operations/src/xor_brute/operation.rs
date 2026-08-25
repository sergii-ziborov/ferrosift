use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Enumerates XOR keys of length 1..=2.
pub struct XorBruteForce {
    spec: OperationSpec,
}

impl XorBruteForce {
    /// Creates the XOR brute-force operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "logic.xor_brute@1",
                display_name: "XOR Brute Force",
                category: "Logic",
                description: "Enumerates XOR keys and optional crib-filtered plaintexts.",
                cyberchef_alias: Some("XOR Brute Force"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("key_length", "Key length in bytes (1 or 2).", 1),
                    integer_argument("sample_length", "Bytes of input to sample.", 100),
                    integer_argument("sample_offset", "Offset into the input sample.", 0),
                    text_argument(
                        "scheme",
                        "Standard, Input differential, or Output differential.",
                        "Standard",
                    ),
                    boolean_argument(
                        "null_preserving",
                        "Skip XOR when byte is 0 or equals key.",
                        false,
                    ),
                    boolean_argument(
                        "print_key",
                        "Include the key hex prefix on each line.",
                        true,
                    ),
                    boolean_argument(
                        "output_as_hex",
                        "Emit sample plaintext as spaced hex.",
                        false,
                    ),
                    text_argument(
                        "crib",
                        "Optional known-plaintext filter (binaryString escapes).",
                        "",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for XorBruteForce {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for XorBruteForce {
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
        // Brute force key length 2 expands heavily; raise effective allowance via budget check inside.
        let text = codec::brute(
            &input,
            &codec::BruteOptions {
                key_length: integer_value(arguments, "key_length")?,
                sample_length: integer_value(arguments, "sample_length")?,
                sample_offset: integer_value(arguments, "sample_offset")?,
                scheme: text_value(arguments, "scheme")?,
                null_preserving: boolean_value(arguments, "null_preserving")?,
                print_key: boolean_value(arguments, "print_key")?,
                output_hex: boolean_value(arguments, "output_as_hex")?,
                crib: text_value(arguments, "crib")?,
            },
            context,
        )?;
        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}
