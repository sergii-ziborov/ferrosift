use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

/// Wraps hexadecimal DER in a PEM block.
pub struct HexToPem {
    spec: OperationSpec,
}

impl HexToPem {
    /// Creates the PEM writer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "asn1.pem.encode@1",
                display_name: "Hex to PEM",
                category: "Parsing",
                description: "Wraps a hexadecimal DER string in a PEM block with the given label.",
                cyberchef_alias: Some("Hex to PEM"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![text_argument(
                    "header",
                    "Label for the BEGIN and END lines.",
                    "CERTIFICATE",
                )],
                inverse: Some("asn1.pem.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for HexToPem {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for HexToPem {
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
        let header = text_value(arguments, "header")?;
        let input = take_text(input)?;
        let output = codec::to_pem(&input, header, context)?;
        context.ensure_active()?;
        Ok(text_output(output))
    }
}

/// Extracts the DER body from every PEM block.
pub struct PemToHex {
    spec: OperationSpec,
}

impl PemToHex {
    /// Creates the PEM reader.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "asn1.pem.decode@1",
                display_name: "PEM to Hex",
                category: "Parsing",
                description: "Extracts the DER body of every PEM block as hexadecimal.",
                cyberchef_alias: Some("PEM to Hex"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: Some("asn1.pem.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for PemToHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for PemToHex {
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
        let output = codec::from_pem(&input, context)?;
        context.ensure_active()?;
        Ok(text_output(output))
    }
}
