use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{take_text, text as text_output};

use super::codec;

/// Builds a text-in / text-out specification for this family.
fn text_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Text,
        UniformSpec {
            id,
            display_name,
            category: "Text",
            description,
            cyberchef_alias: alias,
            arguments,
        },
    )
}

/// Keeps the last n delimited fields, like UNIX `tail`.
pub struct Tail {
    spec: OperationSpec,
}

impl Tail {
    /// Creates the tail operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "data.tail@1",
                "Tail",
                "Gets the last n delimited fields from the input.",
                "Tail",
                vec![
                    text_argument("delimiter", "Field delimiter token.", "Line feed"),
                    integer_argument(
                        "number",
                        "Number of fields to keep; negative drops the first -n.",
                        10,
                    ),
                ],
            ),
        }
    }
}

impl Default for Tail {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Tail {
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
        let output = codec::tail(
            &input,
            text_value(arguments, "delimiter")?,
            integer_value(arguments, "number")?,
            context,
        )?;
        Ok(text_output(output))
    }
}

/// Prefixes each line with its number.
pub struct AddLineNumbers {
    spec: OperationSpec,
}

impl AddLineNumbers {
    /// Creates the add-line-numbers operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.line_numbers.add@1",
                "Add line numbers",
                "Prefixes every line with its number, right-aligned.",
                "Add line numbers",
                vec![integer_argument("offset", "Added to every line number.", 0)],
            ),
        }
    }
}

impl Default for AddLineNumbers {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AddLineNumbers {
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
        let output = codec::add_line_numbers(&input, integer_value(arguments, "offset")?, context)?;
        Ok(text_output(output))
    }
}

/// Strips leading line numbers where they are trivially detectable.
pub struct RemoveLineNumbers {
    spec: OperationSpec,
}

impl RemoveLineNumbers {
    /// Creates the remove-line-numbers operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.line_numbers.remove@1",
                "Remove line numbers",
                "Removes leading line numbers where they are trivially detectable.",
                "Remove line numbers",
                vec![],
            ),
        }
    }
}

impl Default for RemoveLineNumbers {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RemoveLineNumbers {
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
        let input = take_text(input)?;
        Ok(text_output(codec::remove_line_numbers(&input, context)?))
    }
}

/// Pads every line with a repeating filler.
pub struct PadLines {
    spec: OperationSpec,
}

impl PadLines {
    /// Creates the pad-lines operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.pad_lines@1",
                "Pad lines",
                "Adds padding characters to the start or end of every line.",
                "Pad lines",
                vec![
                    text_argument("position", "Where to add padding: Start or End.", "Start"),
                    integer_argument("length", "Number of padding characters.", 5),
                    text_argument("character", "Padding filler.", " "),
                ],
            ),
        }
    }
}

impl Default for PadLines {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for PadLines {
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
        let output = codec::pad_lines(
            &input,
            text_value(arguments, "position")?,
            integer_value(arguments, "length")?,
            text_value(arguments, "character")?,
            context,
        )?;
        Ok(text_output(output))
    }
}
