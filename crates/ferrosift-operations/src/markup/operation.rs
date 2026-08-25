use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

fn text_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Text",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(ValueKind::Text),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// Replaces typographic characters with ASCII equivalents.
pub struct EscapeSmartCharacters {
    spec: OperationSpec,
}

impl EscapeSmartCharacters {
    /// Creates the smart-character folding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.smart.escape@1",
                "Escape Smart Characters",
                "Replaces curly quotes, dashes, arrows and similar with ASCII.",
                "Escape Smart Characters",
                vec![text_argument(
                    "unmappable_characters",
                    "What to do with characters the table does not cover: Include, Remove, or Replace with '.'.",
                    "Include",
                )],
            ),
        }
    }
}

impl Default for EscapeSmartCharacters {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for EscapeSmartCharacters {
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
        let unmappable = codec::unmappable(text_value(arguments, "unmappable_characters")?)?;
        let input = take_text(input)?;
        Ok(text_output(codec::escape_smart(&input, unmappable)))
    }
}

/// Removes HTML tags, keeping the text between them.
pub struct StripHtmlTags {
    spec: OperationSpec,
}

impl StripHtmlTags {
    /// Creates the tag-stripping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.html.strip_tags@1",
                "Strip HTML tags",
                "Removes HTML tags, optionally tidying the whitespace left behind.",
                "Strip HTML tags",
                vec![
                    boolean_argument(
                        "remove_indentation",
                        "Drop leading spaces and tabs after each newline.",
                        true,
                    ),
                    boolean_argument(
                        "remove_excess_line_breaks",
                        "Collapse runs of blank lines into one.",
                        true,
                    ),
                ],
            ),
        }
    }
}

impl Default for StripHtmlTags {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for StripHtmlTags {
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
        let remove_indentation = boolean_value(arguments, "remove_indentation")?;
        let remove_line_breaks = boolean_value(arguments, "remove_excess_line_breaks")?;
        let input = take_text(input)?;
        Ok(text_output(codec::strip_html_tags(
            &input,
            remove_indentation,
            remove_line_breaks,
        )))
    }
}
