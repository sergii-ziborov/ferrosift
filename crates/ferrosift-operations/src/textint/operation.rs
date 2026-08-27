use alloc::string::ToString;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::failure::failed;
use crate::jscompat::delim::is_js_whitespace;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec::{self, Reading};

/// Text as one large integer, and back.
pub struct TextIntegerConversion {
    spec: OperationSpec,
}

impl TextIntegerConversion {
    /// Creates the text-integer conversion.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.text_integer@1",
                display_name: "Text-Integer Conversion",
                category: "Encoding",
                description: "Reads text as a big-endian run of character codes, or writes such a number back as text.",
                cyberchef_alias: Some("Text-Integer Conversion"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "output_format",
                    "How to write the answer: String, Decimal, or Hexadecimal.",
                    "String",
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for TextIntegerConversion {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for TextIntegerConversion {
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
        let format = text_value(arguments, "output_format")?;
        let input = take_text(input)?;
        let trimmed = input.trim_matches(is_js_whitespace);

        let value = match codec::classify(trimmed) {
            Reading::Number(value) => value,
            // Quoted or not, what is left is text. Unquoting first is what
            // makes `"123"` three characters where `123` is a number.
            Reading::Text => codec::text_to_number(codec::unquote(trimmed))
                .ok_or_else(|| failed("encoding.text_integer.beyond_latin1"))?,
        };

        context.ensure_active()?;
        // Anything that is not one of the two number formats is the string
        // one, which is the reference's own fallthrough rather than a listing
        // of the three names its interface offers.
        let written = match format {
            "Decimal" => value.to_string(),
            "Hexadecimal" => codec::to_hexadecimal(&value),
            _ => codec::number_to_text(&value),
        };
        Ok(text_output(written))
    }
}
