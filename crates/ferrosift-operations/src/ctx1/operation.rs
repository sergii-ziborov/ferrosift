use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

/// Obfuscates a password into the Citrix CTX1 form.
pub struct CitrixCtx1Encode {
    spec: OperationSpec,
}

impl CitrixCtx1Encode {
    /// Creates the CTX1 encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.ctx1.encode@1",
                display_name: "Citrix CTX1 Encode",
                category: "Encoding",
                description: "Obfuscates a password into the Citrix CTX1 form. This is not encryption and has no key.",
                cyberchef_alias: Some("Citrix CTX1 Encode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("encoding.ctx1.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for CitrixCtx1Encode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for CitrixCtx1Encode {
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
        context.ensure_active()?;
        Ok(bytes_output(codec::encode(&input)))
    }
}

/// Recovers a password from the Citrix CTX1 form.
pub struct CitrixCtx1Decode {
    spec: OperationSpec,
}

impl CitrixCtx1Decode {
    /// Creates the CTX1 decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.ctx1.decode@1",
                display_name: "Citrix CTX1 Decode",
                category: "Encoding",
                description: "Recovers a password from the Citrix CTX1 form.",
                cyberchef_alias: Some("Citrix CTX1 Decode"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: Some("encoding.ctx1.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for CitrixCtx1Decode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for CitrixCtx1Decode {
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
        context.ensure_active()?;
        Ok(text_output(codec::decode(&input)?))
    }
}
