use alloc::boxed::Box;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError, StreamSession, Streamable};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build, incremental};
use crate::stream::HexSession;

use super::delimiter::EncodeDelimiter;
use super::{codec, delimiter};

/// Encodes bytes as lower-case hexadecimal text.
pub struct ToHex {
    spec: OperationSpec,
}

impl ToHex {
    /// Creates the hexadecimal encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: incremental(build(SpecDefinition {
                id: "encoding.hex.encode@1",
                display_name: "To Hex",
                category: "Encoding",
                description: "Encodes bytes as lower-case hexadecimal text.",
                cyberchef_alias: Some("To Hex"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Hex byte delimiter.", "Space"),
                    integer_argument("bytes_per_line", "Bytes emitted per line.", 0),
                ],
                inverse: Some("encoding.hex.decode@1"),
                classifications: None,
            })),
        }
    }
}

impl Default for ToHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToHex {
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
        let delimiter = delimiter::encode(text_value(arguments, "delimiter")?)?;
        let line_size = nonnegative_usize(integer_value(arguments, "bytes_per_line")?)?;
        let output = codec::encode(&input, delimiter, line_size, context)?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

impl Streamable for ToHex {
    fn start(
        &self,
        arguments: &Arguments,
        _context: &OperationContext<'_>,
    ) -> Result<Option<Box<dyn StreamSession + '_>>, OperationError> {
        // Only the contiguous form. A delimiter or a line width makes the
        // output depend on *where* the last byte is, and a session does not
        // know that until it ends — it would have to hold a byte back and
        // decide at `finish`, which is implementable and is not implemented
        // here rather than implemented approximately. Everything else answers
        // `None` and the caller uses the buffered path, which is what `None`
        // is for.
        let delimiter = delimiter::encode(text_value(arguments, "delimiter")?)?;
        let line_size = nonnegative_usize(integer_value(arguments, "bytes_per_line")?)?;
        if line_size != 0 || !matches!(delimiter, EncodeDelimiter::Suffix("")) {
            return Ok(None);
        }
        Ok(Some(Box::new(HexSession::new())))
    }
}

/// Decodes hexadecimal text into bytes.
pub struct FromHex {
    spec: OperationSpec,
}

impl FromHex {
    /// Creates the hexadecimal decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.hex.decode@1",
                display_name: "From Hex",
                category: "Encoding",
                description: "Decodes validated hexadecimal text into bytes.",
                cyberchef_alias: Some("From Hex"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![text_argument(
                    "delimiter",
                    "Hex byte delimiter or automatic detection.",
                    "Auto",
                )],
                inverse: Some("encoding.hex.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromHex {
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
        let delimiter = delimiter::decode(text_value(arguments, "delimiter")?)?;
        codec::decode(&input.text, delimiter, context).map(Value::Bytes)
    }
}

fn nonnegative_usize(value: i128) -> Result<usize, OperationError> {
    usize::try_from(value).map_err(|_| delimiter::failed("encoding.hex.invalid_line_width"))
}
