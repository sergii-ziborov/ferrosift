use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, integer_argument, integer_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Reads Type-Length-Value records and reports them as JSON.
pub struct ParseTlv {
    spec: OperationSpec,
}

impl ParseTlv {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "parsing.tlv@1",
                display_name: "Parse TLV",
                category: "Parsing",
                description: "Reads Type-Length-Value records into a JSON list.",
                cyberchef_alias: Some("Parse TLV"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument("key_size", "Bytes of type or key; zero omits it.", 1),
                    integer_argument("length_size", "Bytes of length.", 1),
                    boolean_argument("use_ber", "Read the length as BER.", false),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ParseTlv {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ParseTlv {
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
        let key_size = i64::try_from(integer_value(arguments, "key_size")?).unwrap_or(0);
        let length_size = i64::try_from(integer_value(arguments, "length_size")?).unwrap_or(0);
        let ber = boolean_value(arguments, "use_ber")?;
        Ok(Value::Text(TextValue {
            text: codec::parse(&input, key_size, length_size, ber, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}