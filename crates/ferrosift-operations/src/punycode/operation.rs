use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Which direction and which of the two surfaces the operation exposes.
#[derive(Clone, Copy)]
enum Direction {
    Encode,
    Decode,
}

/// Punycode, either as a bare label transform or across a whole domain.
pub struct Punycode {
    spec: OperationSpec,
    direction: Direction,
}

impl Punycode {
    /// Creates the encoding half.
    #[must_use]
    pub fn encode() -> Self {
        Self::new(Direction::Encode)
    }

    /// Creates the decoding half.
    #[must_use]
    pub fn decode() -> Self {
        Self::new(Direction::Decode)
    }

    fn new(direction: Direction) -> Self {
        let (id, name, alias, inverse, description) = match direction {
            Direction::Encode => (
                "encoding.punycode.encode@1",
                "To Punycode",
                "To Punycode",
                "encoding.punycode.decode@1",
                "Encodes Unicode as Punycode, optionally as a domain name.",
            ),
            Direction::Decode => (
                "encoding.punycode.decode@1",
                "From Punycode",
                "From Punycode",
                "encoding.punycode.encode@1",
                "Decodes Punycode to Unicode, optionally as a domain name.",
            ),
        };
        Self {
            spec: build(SpecDefinition {
                id,
                display_name: name,
                category: "Encoding",
                description,
                cyberchef_alias: Some(alias),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![boolean_argument(
                    "internationalised_domain_name",
                    "Treat the input as a domain name rather than one label.",
                    false,
                )],
                inverse: Some(inverse),
                classifications: None,
            }),
            direction,
        }
    }
}

impl Operation for Punycode {
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
        let domain = boolean_value(arguments, "internationalised_domain_name")?;
        let text = match (self.direction, domain) {
            (Direction::Encode, false) => codec::encode(&input.text, context)?,
            (Direction::Encode, true) => codec::to_ascii(&input.text, context)?,
            (Direction::Decode, false) => codec::decode(&input.text, context)?,
            (Direction::Decode, true) => codec::to_unicode(&input.text, context)?,
        };
        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}
