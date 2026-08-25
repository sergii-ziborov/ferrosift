use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

/// Encodes bytes as quoted-printable text.
pub struct ToQuotedPrintable {
    spec: OperationSpec,
}

impl ToQuotedPrintable {
    /// Creates the quoted-printable encoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.quoted_printable.encode@1",
                display_name: "To Quoted Printable",
                category: "Encoding",
                description: "Encodes bytes as quoted-printable text, breaking lines at 76 characters.",
                cyberchef_alias: Some("To Quoted Printable"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: Some("encoding.quoted_printable.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToQuotedPrintable {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToQuotedPrintable {
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
        Ok(text_output(codec::encode(&input)))
    }
}

/// Decodes quoted-printable text.
pub struct FromQuotedPrintable {
    spec: OperationSpec,
}

impl FromQuotedPrintable {
    /// Creates the quoted-printable decoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.quoted_printable.decode@1",
                display_name: "From Quoted Printable",
                category: "Encoding",
                description: "Decodes quoted-printable text into the bytes it represents.",
                cyberchef_alias: Some("From Quoted Printable"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("encoding.quoted_printable.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromQuotedPrintable {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromQuotedPrintable {
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
        Ok(bytes_output(codec::decode(&input)?))
    }
}
