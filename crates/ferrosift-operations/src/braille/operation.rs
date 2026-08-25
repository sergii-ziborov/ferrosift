use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

fn spec_for(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    kind: ValueKind,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Encoding",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(kind),
        output: ValueConstraint::Exact(kind),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// Transcribes ASCII into braille cells.
pub struct ToBraille {
    spec: OperationSpec,
}

impl ToBraille {
    /// Creates the braille encoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.braille.encode@1",
                "To Braille",
                "Transcribes text into six-dot braille cells.",
                "To Braille",
                ValueKind::Text,
                vec![],
            ),
        }
    }
}

impl Default for ToBraille {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBraille {
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
        Ok(text_output(codec::to_braille(&input)))
    }
}

/// Transcribes braille cells back into text.
pub struct FromBraille {
    spec: OperationSpec,
}

impl FromBraille {
    /// Creates the braille decoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.braille.decode@1",
                "From Braille",
                "Transcribes six-dot braille cells back into text.",
                "From Braille",
                ValueKind::Text,
                vec![],
            ),
        }
    }
}

impl Default for FromBraille {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBraille {
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
        Ok(text_output(codec::from_braille(&input)))
    }
}

/// Decorates text with combining underline and strikethrough marks.
pub struct UnicodeTextFormat {
    spec: OperationSpec,
}

impl UnicodeTextFormat {
    /// Creates the text decoration operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "text.unicode.format@1",
                "Unicode Text Format",
                "Adds combining underline or strikethrough marks to the input.",
                "Unicode Text Format",
                ValueKind::Bytes,
                vec![
                    boolean_argument("underline", "Add a combining low line.", false),
                    boolean_argument(
                        "strikethrough",
                        "Add a combining long stroke overlay.",
                        false,
                    ),
                ],
            ),
        }
    }
}

impl Default for UnicodeTextFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UnicodeTextFormat {
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
        let underline = boolean_value(arguments, "underline")?;
        let strikethrough = boolean_value(arguments, "strikethrough")?;
        let input = take_bytes(input)?;
        Ok(bytes_output(codec::unicode_text_format(
            &input,
            underline,
            strikethrough,
        )))
    }
}
