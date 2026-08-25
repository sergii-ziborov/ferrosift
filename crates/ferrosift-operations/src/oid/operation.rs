use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

fn spec_for(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    inverse: &'static str,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Parsing",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(ValueKind::Text),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments: vec![],
        inverse: Some(inverse),
        classifications: None,
    })
}

/// Encodes a dotted object identifier as hexadecimal.
pub struct ObjectIdentifierToHex {
    spec: OperationSpec,
}

impl ObjectIdentifierToHex {
    /// Creates the OID encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "asn1.oid.encode@1",
                "Object Identifier to Hex",
                "Encodes a dotted ASN.1 object identifier as its hexadecimal DER value.",
                "Object Identifier to Hex",
                "asn1.oid.decode@1",
            ),
        }
    }
}

impl Default for ObjectIdentifierToHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ObjectIdentifierToHex {
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
        Ok(text_output(codec::to_hex(&input)?))
    }
}

/// Decodes a hexadecimal DER value into a dotted object identifier.
pub struct HexToObjectIdentifier {
    spec: OperationSpec,
}

impl HexToObjectIdentifier {
    /// Creates the OID decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "asn1.oid.decode@1",
                "Hex to Object Identifier",
                "Decodes a hexadecimal DER value into a dotted ASN.1 object identifier.",
                "Hex to Object Identifier",
                "asn1.oid.encode@1",
            ),
        }
    }
}

impl Default for HexToObjectIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for HexToObjectIdentifier {
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
        Ok(text_output(codec::from_hex(&input)?))
    }
}
