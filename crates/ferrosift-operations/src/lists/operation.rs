use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::jscompat::delim::char_rep;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

/// Removes repeated entries from a delimited list.
pub struct Unique {
    spec: OperationSpec,
}

impl Unique {
    /// Creates the deduplication operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "list.unique@1",
                display_name: "Unique",
                category: "Shaping",
                description: "Removes repeated entries from a delimited list, optionally counting them.",
                cyberchef_alias: Some("Unique"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Separator between entries.", "Line feed"),
                    boolean_argument(
                        "display_count",
                        "Prefix each entry with how many times it occurred.",
                        false,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Unique {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Unique {
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
        let delimiter = char_rep(
            text_value(arguments, "delimiter")?,
            "list.unique.unknown_delimiter",
        )?;
        let counted = boolean_value(arguments, "display_count")?;
        let input = take_text(input)?;

        context.ensure_active()?;
        Ok(text_output(if counted {
            codec::unique_with_counts(&input, delimiter)
        } else {
            codec::unique(&input, delimiter)
        }))
    }
}

/// Re-delimits a list.
pub struct Split {
    spec: OperationSpec,
}

impl Split {
    /// Creates the re-delimiting operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "list.split@1",
                display_name: "Split",
                category: "Shaping",
                description: "Splits the input on one delimiter and rejoins it with another.",
                cyberchef_alias: Some("Split"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    // Literal text, not delimiter names: the reference offers
                    // these two as editable fields, so `\n` here is a
                    // backslash and an `n` rather than a line feed.
                    text_argument("split_delimiter", "Literal text to split on.", ","),
                    text_argument("join_delimiter", "Literal text to join with.", "\\n"),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Split {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Split {
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
        let from = text_value(arguments, "split_delimiter")?;
        let to = text_value(arguments, "join_delimiter")?;
        let input = take_text(input)?;
        context.ensure_active()?;
        Ok(text_output(codec::respan(&input, from, to)))
    }
}
