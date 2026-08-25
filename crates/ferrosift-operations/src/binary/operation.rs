use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Encodes bytes as binary digit text.
pub struct ToBinary {
    spec: OperationSpec,
}

impl ToBinary {
    /// Creates the binary encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.binary.encode@1",
                display_name: "To Binary",
                category: "Encoding",
                description: "Encodes bytes as delimited binary digits.",
                cyberchef_alias: Some("To Binary"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Digit group delimiter.", "Space"),
                    integer_argument("byte_length", "Digits emitted per byte.", 8),
                ],
                inverse: Some("encoding.binary.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBinary {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBinary {
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
        let width = byte_length(arguments)?;
        let output = codec::encode(&input, text_value(arguments, "delimiter")?, width, context)?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes binary digit text into bytes.
pub struct FromBinary {
    spec: OperationSpec,
}

impl FromBinary {
    /// Creates the binary decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.binary.decode@1",
                display_name: "From Binary",
                category: "Encoding",
                description: "Decodes delimited binary digits into bytes.",
                cyberchef_alias: Some("From Binary"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument("delimiter", "Digit group delimiter.", "Space"),
                    integer_argument("byte_length", "Digits consumed per byte.", 8),
                ],
                inverse: Some("encoding.binary.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBinary {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBinary {
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
        let width = byte_length(arguments)?;
        codec::decode(
            &input.text,
            text_value(arguments, "delimiter")?,
            width,
            context,
        )
        .map(Value::Bytes)
    }
}

/// The reference UI bounds the byte length to 1..=256; values outside that
/// range fail with a stable code instead of the runtime's silent fallback.
fn byte_length(arguments: &Arguments) -> Result<usize, OperationError> {
    let value = integer_value(arguments, "byte_length")?;
    if (1..=256).contains(&value) {
        usize::try_from(value).map_err(|_| failed(codec::INVALID_BYTE_LENGTH))
    } else {
        Err(failed(codec::INVALID_BYTE_LENGTH))
    }
}
