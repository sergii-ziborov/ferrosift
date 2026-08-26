use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::jscompat::escape::parse_escaped_chars;
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Renders delimited text as a table.
pub struct ToTable {
    spec: OperationSpec,
}

impl ToTable {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "shaping.to_table@1",
                display_name: "To Table",
                category: "Shaping",
                description: "Renders delimited text as an ASCII, HTML, or Markdown table.",
                cyberchef_alias: Some("To Table"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Markup),
                arguments: vec![
                    text_argument("cell_delimiters", "Characters that end a cell.", ","),
                    text_argument("row_delimiters", "Characters that end a row.", "\\r\\n"),
                    boolean_argument(
                        "first_row_header",
                        "Treat the first row as a header.",
                        false,
                    ),
                    text_argument("format", "ASCII, HTML, or Markdown.", "ASCII"),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ToTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToTable {
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
        // Each character of the argument is a delimiter in its own right, so
        // `,;` splits on either rather than on the pair.
        let cells = parse_escaped_chars(text_value(arguments, "cell_delimiters")?);
        let rows = parse_escaped_chars(text_value(arguments, "row_delimiters")?);
        let header = boolean_value(arguments, "first_row_header")?;
        let format = codec::format(text_value(arguments, "format")?);

        Ok(Value::Markup(codec::render(
            &input.text,
            &cells,
            &rows,
            header,
            format,
            context,
        )?))
    }
}
