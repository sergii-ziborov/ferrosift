use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

fn spec_for(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    input: ValueKind,
    output: ValueKind,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Encoding",
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(input),
        output: ValueConstraint::Exact(output),
        arguments: vec![],
        inverse: None,
        classifications: None,
    })
}

/// Encodes an integer as a base-128 `VarInt`.
pub struct VarIntEncode {
    spec: OperationSpec,
}

impl VarIntEncode {
    /// Creates the `VarInt` encoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.varint.encode@1",
                "VarInt Encode",
                "Encodes a non-negative integer as a base-128 VarInt.",
                "VarInt Encode",
                ValueKind::Text,
                ValueKind::Bytes,
            ),
        }
    }
}

impl Default for VarIntEncode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for VarIntEncode {
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
        Ok(bytes_output(codec::encode(&input)?))
    }
}

/// Decodes a base-128 `VarInt`.
pub struct VarIntDecode {
    spec: OperationSpec,
}

impl VarIntDecode {
    /// Creates the `VarInt` decoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.varint.decode@1",
                "VarInt Decode",
                "Decodes a base-128 VarInt into its decimal digits.",
                "VarInt Decode",
                ValueKind::Bytes,
                ValueKind::Text,
            ),
        }
    }
}

impl Default for VarIntDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for VarIntDecode {
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
        Ok(text_output(codec::decode(&input)?))
    }
}
