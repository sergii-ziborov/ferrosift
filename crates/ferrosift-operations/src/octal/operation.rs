use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encodes bytes as octal ordinal text.
pub struct ToOctal {
    spec: OperationSpec,
}

impl ToOctal {
    /// Creates the octal encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.octal.encode@1",
                display_name: "To Octal",
                category: "Encoding",
                description: "Encodes bytes as delimited octal ordinals.",
                cyberchef_alias: Some("To Octal"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument("delimiter", "Ordinal delimiter.", "Space")],
                inverse: Some("encoding.octal.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToOctal {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToOctal {
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
        let output = codec::encode(&input, text_value(arguments, "delimiter")?, context)?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes octal ordinal text into bytes.
pub struct FromOctal {
    spec: OperationSpec,
}

impl FromOctal {
    /// Creates the octal decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.octal.decode@1",
                display_name: "From Octal",
                category: "Encoding",
                description: "Decodes delimited octal ordinals into bytes.",
                cyberchef_alias: Some("From Octal"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![text_argument("delimiter", "Ordinal delimiter.", "Space")],
                inverse: Some("encoding.octal.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromOctal {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromOctal {
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
        if input.text.is_empty() {
            return Ok(Value::Bytes(Vec::new()));
        }
        codec::decode(&input.text, text_value(arguments, "delimiter")?, context).map(Value::Bytes)
    }
}
