use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Which direction of the Bifid cipher an instance runs.
#[derive(Clone, Copy)]
enum Direction {
    Encode,
    Decode,
}

/// The Bifid cipher: a Polybius square plus fractionating transposition.
pub struct BifidCipher {
    spec: OperationSpec,
    direction: Direction,
}

impl BifidCipher {
    /// Creates the Bifid encoder.
    #[must_use]
    pub fn encode() -> Self {
        Self {
            spec: bifid_spec(
                "cipher.bifid.encode@1",
                "Bifid Cipher Encode",
                "Encodes text with a Polybius square and fractionating transposition.",
                "cipher.bifid.decode@1",
            ),
            direction: Direction::Encode,
        }
    }

    /// Creates the Bifid decoder.
    #[must_use]
    pub fn decode() -> Self {
        Self {
            spec: bifid_spec(
                "cipher.bifid.decode@1",
                "Bifid Cipher Decode",
                "Reverses the Bifid transposition and reads the square.",
                "cipher.bifid.encode@1",
            ),
            direction: Direction::Decode,
        }
    }
}

fn bifid_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    inverse: &'static str,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Ciphers",
        description,
        cyberchef_alias: Some(display_name),
        input: ValueConstraint::Exact(ValueKind::Text),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments: vec![text_argument(
            "keyword",
            "Letters that lead the Polybius square.",
            "",
        )],
        inverse: Some(inverse),
        classifications: None,
    })
}

impl Operation for BifidCipher {
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
        let keyword = text_value(arguments, "keyword")?;
        let input = crate::value::take_text_value(input)?;
        let output = match self.direction {
            Direction::Encode => codec::encode(&input.text, keyword, context)?,
            Direction::Decode => codec::decode(&input.text, keyword, context)?,
        };
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}
