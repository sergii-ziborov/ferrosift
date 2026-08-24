//! Operation wrappers for Modhex.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{bytes, take_bytes, take_text, text};

use super::codec;

/// Encodes bytes as modhex.
pub struct ToModhex {
    spec: OperationSpec,
}

impl ToModhex {
    /// Creates the modhex encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.modhex.encode@1",
                display_name: "To Modhex",
                category: "Encoding",
                description: "Encodes bytes as modhex, the keyboard-safe hex alphabet.",
                cyberchef_alias: Some("To Modhex"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("delimiter", "Byte delimiter token.", "Space"),
                    integer_argument("bytes_per_line", "Wrap after this many bytes.", 0),
                ],
                inverse: Some("encoding.modhex.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToModhex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToModhex {
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
        let delimiter = text_value(arguments, "delimiter")?;
        let line_size = integer_value(arguments, "bytes_per_line")?;
        let input = take_bytes(input)?;
        Ok(text(codec::encode(&input, delimiter, line_size, context)?))
    }
}

/// Decodes modhex back into bytes.
pub struct FromModhex {
    spec: OperationSpec,
}

impl FromModhex {
    /// Creates the modhex decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.modhex.decode@1",
                display_name: "From Modhex",
                category: "Encoding",
                description: "Decodes modhex back into bytes.",
                cyberchef_alias: Some("From Modhex"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![text_argument("delimiter", "Byte delimiter token.", "Auto")],
                inverse: Some("encoding.modhex.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromModhex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromModhex {
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
        let delimiter = text_value(arguments, "delimiter")?;
        let input = take_text(input)?;
        Ok(bytes(codec::decode(&input, delimiter, context)?))
    }
}
