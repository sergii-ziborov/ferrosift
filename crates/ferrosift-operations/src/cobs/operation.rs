use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes};

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
        category: "Encoding",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(ValueKind::Bytes),
        output: ValueConstraint::Exact(ValueKind::Bytes),
        arguments: vec![],
        inverse: Some(inverse),
        classifications: None,
    })
}

/// Frames bytes so the result contains no zero byte.
pub struct ToCobs {
    spec: OperationSpec,
}

impl ToCobs {
    /// Creates the COBS encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.cobs.encode@1",
                "To COBS",
                "Encodes bytes so that the result contains no zero byte, which lets zero delimit frames.",
                "To COBS",
                "encoding.cobs.decode@1",
            ),
        }
    }
}

impl Default for ToCobs {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToCobs {
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
        Ok(bytes_output(codec::encode(&input)))
    }
}

/// Recovers the original bytes from a COBS frame.
pub struct FromCobs {
    spec: OperationSpec,
}

impl FromCobs {
    /// Creates the COBS decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.cobs.decode@1",
                "From COBS",
                "Recovers the original bytes from a COBS frame.",
                "From COBS",
                "encoding.cobs.encode@1",
            ),
        }
    }
}

impl Default for FromCobs {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromCobs {
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
        Ok(bytes_output(codec::decode(&input)?))
    }
}
