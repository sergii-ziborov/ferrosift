use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Decodes Microsoft's encoded-script format.
pub struct MicrosoftScriptDecoder {
    spec: OperationSpec,
}

impl MicrosoftScriptDecoder {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.microsoft_script.decode@1",
                display_name: "Microsoft Script Decoder",
                category: "Encoding",
                description: "Decodes an encoded VBScript or JScript file.",
                cyberchef_alias: Some("Microsoft Script Decoder"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for MicrosoftScriptDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for MicrosoftScriptDecoder {
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
        let input = crate::value::take_text_value(input)?;
        Ok(Value::Text(TextValue {
            text: codec::decode(&input.text, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
