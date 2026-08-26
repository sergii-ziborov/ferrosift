use alloc::{collections::BTreeSet, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationClassification, OperationSpec, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    integer_argument, integer_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::codec_bytes::{decode_input, encode_output, toggle_bytes};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// RC4 with the leading keystream discarded.
pub struct Rc4Drop {
    spec: OperationSpec,
}

impl Rc4Drop {
    /// Creates the RC4 Drop operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "crypto.rc4_drop@1",
                display_name: "RC4 Drop",
                category: "Ciphers",
                description: "RC4 with the weak leading keystream words discarded.",
                cyberchef_alias: Some("RC4 Drop"),
                input: ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text])),
                output: ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text])),
                arguments: vec![
                    map_argument(
                        "passphrase",
                        "RC4 key as CyberChef toggleString.",
                        toggle_string_default("UTF8", ""),
                    ),
                    text_argument(
                        "input_format",
                        "Input format: Latin1, UTF8, Hex, or Base64.",
                        "Latin1",
                    ),
                    text_argument(
                        "output_format",
                        "Output format: Latin1, UTF8, Hex, or Base64.",
                        "Latin1",
                    ),
                    integer_argument("drop", "Number of 32-bit keystream words to discard.", 192),
                ],
                inverse: Some("crypto.rc4_drop@1"),
                classifications: Some(&[
                    OperationClassification::Legacy,
                    OperationClassification::Unsafe,
                ]),
            }),
        }
    }
}

impl Default for Rc4Drop {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Rc4Drop {
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
        let (opt, string) = toggle_string_parts(map_value(arguments, "passphrase")?)?;
        let key = toggle_bytes(opt, string)?;
        let input_format = text_value(arguments, "input_format")?;
        let output_format = text_value(arguments, "output_format")?;
        // A count at or below zero drops nothing: the reference counts down to
        // zero, so a negative figure simply never enters the loop.
        let drop = u64::try_from(integer_value(arguments, "drop")?).unwrap_or(0);
        let format = match input_format {
            "Latin1" => "Raw",
            other => other,
        };
        let message = decode_input(input, format)?;
        let output = codec::apply_dropping(&message, &key, drop, context)?;
        let out_format = match output_format {
            "Latin1" => "Raw",
            other => other,
        };
        encode_output(&output, out_format)
    }
}
