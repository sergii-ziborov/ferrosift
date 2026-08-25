use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encodes text as Base92.
pub struct ToBase92 {
    spec: OperationSpec,
}

impl ToBase92 {
    /// Creates the Base92 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base92.encode@1",
                display_name: "To Base92",
                category: "Encoding",
                description: "Encodes the input as Base92 text.",
                cyberchef_alias: Some("To Base92"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: Some("encoding.base92.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBase92 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBase92 {
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
        let input = crate::value::take_text_value(input)?;
        Ok(Value::Text(TextValue {
            text: codec::encode(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes Base92 text into bytes.
pub struct FromBase92 {
    spec: OperationSpec,
}

impl FromBase92 {
    /// Creates the Base92 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.base92.decode@1",
                display_name: "From Base92",
                category: "Encoding",
                description: "Decodes Base92 text into bytes.",
                cyberchef_alias: Some("From Base92"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("encoding.base92.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBase92 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBase92 {
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
        let input = crate::value::take_text_value(input)?;
        if input.text.is_empty() {
            return Ok(Value::Bytes(Vec::new()));
        }
        codec::decode(&input.text, context).map(Value::Bytes)
    }
}
