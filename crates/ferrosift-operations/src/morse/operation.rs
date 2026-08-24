//! Operation wrappers for Morse code.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{take_text, text};

use super::codec;

/// Encodes text as Morse.
pub struct ToMorseCode {
    spec: OperationSpec,
}

impl ToMorseCode {
    /// Creates the Morse encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Text,
                UniformSpec {
                    id: "encoding.morse.encode@1",
                    display_name: "To Morse Code",
                    category: "Encoding",
                    description: "Encodes text as Morse code.",
                    cyberchef_alias: "To Morse Code",
                    arguments: vec![
                        text_argument("format_options", "Dash and dot rendering.", "-/."),
                        text_argument("letter_delimiter", "Between letters.", "Space"),
                        text_argument("word_delimiter", "Between words.", "Line feed"),
                    ],
                },
            ),
        }
    }
}

impl Default for ToMorseCode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToMorseCode {
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
        let format = text_value(arguments, "format_options")?;
        let letter = text_value(arguments, "letter_delimiter")?;
        let word = text_value(arguments, "word_delimiter")?;
        let input = take_text(input)?;
        Ok(text(codec::encode(&input, format, letter, word, context)?))
    }
}

/// Decodes Morse back into text.
pub struct FromMorseCode {
    spec: OperationSpec,
}

impl FromMorseCode {
    /// Creates the Morse decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Text,
                UniformSpec {
                    id: "encoding.morse.decode@1",
                    display_name: "From Morse Code",
                    category: "Encoding",
                    description: "Decodes Morse code back into text.",
                    cyberchef_alias: "From Morse Code",
                    arguments: vec![
                        text_argument("letter_delimiter", "Between letters.", "Space"),
                        text_argument("word_delimiter", "Between words.", "Line feed"),
                    ],
                },
            ),
        }
    }
}

impl Default for FromMorseCode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromMorseCode {
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
        let letter = text_value(arguments, "letter_delimiter")?;
        let word = text_value(arguments, "word_delimiter")?;
        let input = take_text(input)?;
        Ok(text(codec::decode(&input, letter, word, context)?))
    }
}
