use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueKind};

use crate::args::{boolean_argument, boolean_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{bytes, take_bytes, take_text, text};

use super::codec;

/// Argument names in the reference's own order, paired with `STRIPPABLE`.
const WHITESPACE_ARGUMENTS: [&str; 6] = [
    "spaces",
    "carriage_returns",
    "line_feeds",
    "tabs",
    "form_feeds",
    "full_stops",
];

/// Removes selected whitespace characters, and optionally full stops.
pub struct RemoveWhitespace {
    spec: OperationSpec,
}

impl RemoveWhitespace {
    /// Creates the remove-whitespace operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Text,
                UniformSpec {
                    id: "text.remove_whitespace@1",
                    display_name: "Remove whitespace",
                    category: "Text",
                    description: "Removes selected whitespace characters from the input.",
                    cyberchef_alias: "Remove whitespace",
                    arguments: vec![
                        boolean_argument("spaces", "Remove spaces.", true),
                        boolean_argument("carriage_returns", "Remove carriage returns.", true),
                        boolean_argument("line_feeds", "Remove line feeds.", true),
                        boolean_argument("tabs", "Remove tabs.", true),
                        boolean_argument("form_feeds", "Remove form feeds.", true),
                        boolean_argument("full_stops", "Remove full stops.", false),
                    ],
                },
            ),
        }
    }
}

impl Default for RemoveWhitespace {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RemoveWhitespace {
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
        let mut selection = Vec::new();
        for (name, character) in WHITESPACE_ARGUMENTS.iter().zip(codec::STRIPPABLE) {
            if boolean_value(arguments, name)? {
                selection.push(character);
            }
        }
        Ok(text(codec::remove_whitespace(&input, &selection, context)?))
    }
}

/// Removes every zero byte.
pub struct RemoveNullBytes {
    spec: OperationSpec,
}

impl RemoveNullBytes {
    /// Creates the remove-null-bytes operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Bytes,
                UniformSpec {
                    id: "data.remove_null_bytes@1",
                    display_name: "Remove null bytes",
                    category: "Data",
                    description: "Removes every 0x00 byte from the input.",
                    cyberchef_alias: "Remove null bytes",
                    arguments: vec![],
                },
            ),
        }
    }
}

impl Default for RemoveNullBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RemoveNullBytes {
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
        let input = take_bytes(input)?;
        Ok(bytes(codec::remove_null_bytes(&input, context)?))
    }
}
