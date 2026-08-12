use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encodes bytes as decimal ordinal text.
pub struct ToDecimal {
    spec: OperationSpec,
}

impl ToDecimal {
    /// Creates the decimal encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.decimal.encode@1",
                display_name: "To Decimal",
                category: "Encoding",
                description: "Encodes bytes as delimited decimal ordinals.",
                cyberchef_alias: Some("To Decimal"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Ordinal delimiter.", "Space"),
                    boolean_argument(
                        "support_signed",
                        "Emit bytes as signed two's-complement values.",
                        false,
                    ),
                ],
                inverse: Some("encoding.decimal.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToDecimal {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToDecimal {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        let output = codec::encode(
            &input,
            text_value(arguments, "delimiter")?,
            boolean_value(arguments, "support_signed")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes decimal ordinal text into bytes.
pub struct FromDecimal {
    spec: OperationSpec,
}

impl FromDecimal {
    /// Creates the decimal decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.decimal.decode@1",
                display_name: "From Decimal",
                category: "Encoding",
                description: "Decodes delimited decimal ordinals into bytes.",
                cyberchef_alias: Some("From Decimal"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument("delimiter", "Ordinal delimiter or Auto detection.", "Space"),
                    boolean_argument(
                        "support_signed",
                        "Accept signed two's-complement values.",
                        false,
                    ),
                ],
                inverse: Some("encoding.decimal.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromDecimal {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromDecimal {
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
        let Value::Text(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        codec::decode(
            &input.text,
            text_value(arguments, "delimiter")?,
            boolean_value(arguments, "support_signed")?,
            context,
        )
        .map(Value::Bytes)
    }
}
