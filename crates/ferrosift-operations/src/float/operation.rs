use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::failure::failed;
use crate::jscompat::delim::char_rep;
use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec::{self, Width};

fn arguments() -> vec::Vec<ArgumentSpec> {
    vec![
        text_argument(
            "endianness",
            "Byte order of each value: Big Endian or Little Endian.",
            "Big Endian",
        ),
        text_argument(
            "size",
            "Precision: Float (4 bytes) or Double (8 bytes).",
            "Float (4 bytes)",
        ),
        text_argument("delimiter", "Separator between values.", "Space"),
    ]
}

/// Reads the shared arguments.
fn settings(arguments: &Arguments) -> Result<(Width, bool, &'static str), OperationError> {
    let endianness = text_value(arguments, "endianness")?;
    let size = text_value(arguments, "size")?;
    let delimiter = text_value(arguments, "delimiter")?;

    let width = match size {
        "Float (4 bytes)" => Width::Single,
        "Double (8 bytes)" => Width::Double,
        _ => return Err(failed("encoding.float.unknown_size")),
    };
    let little_endian = match endianness {
        "Big Endian" => false,
        "Little Endian" => true,
        _ => return Err(failed("encoding.float.unknown_endianness")),
    };
    let delimiter = char_rep(delimiter, "encoding.float.unknown_delimiter")?;
    Ok((width, little_endian, delimiter))
}

/// Packs decimal numbers into IEEE-754 bytes.
pub struct FromFloat {
    spec: OperationSpec,
}

impl FromFloat {
    /// Creates the float packer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.float.decode@1",
                display_name: "From Float",
                category: "Encoding",
                description: "Packs decimal numbers into IEEE-754 floating-point bytes.",
                cyberchef_alias: Some("From Float"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: arguments(),
                inverse: Some("encoding.float.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromFloat {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromFloat {
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
        let input = take_text(input)?;
        // Empty input short-circuits before the arguments are read, which is
        // the reference's order and means a bad argument goes unreported here.
        if input.is_empty() {
            return Ok(bytes_output(vec::Vec::new()));
        }
        let (width, little_endian, delimiter) = settings(arguments)?;
        let values = codec::parse_all(&input, delimiter);
        context.ensure_active()?;
        Ok(bytes_output(codec::encode(&values, width, little_endian)))
    }
}

/// Reads IEEE-754 bytes back into decimal numbers.
pub struct ToFloat {
    spec: OperationSpec,
}

impl ToFloat {
    /// Creates the float unpacker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.float.encode@1",
                display_name: "To Float",
                category: "Encoding",
                description: "Reads IEEE-754 floating-point bytes as decimal numbers.",
                cyberchef_alias: Some("To Float"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: arguments(),
                inverse: Some("encoding.float.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToFloat {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToFloat {
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
        let input = take_bytes(input)?;
        let (width, little_endian, delimiter) = settings(arguments)?;
        // A partial trailing value is refused rather than dropped: the bytes
        // do not say what the missing ones were, and guessing zero would put a
        // number in the output that was never in the input.
        if !input.len().is_multiple_of(width.size()) {
            return Err(failed("encoding.float.ragged_input"));
        }
        let values = codec::decode(&input, width, little_endian);
        context.ensure_active()?;
        Ok(text_output(if values.is_empty() {
            String::new()
        } else {
            codec::render_all(&values, delimiter)
        }))
    }
}
