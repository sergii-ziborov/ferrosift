use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Decompresses an LZNT1 stream.
pub struct Lznt1Decompress {
    spec: OperationSpec,
}

impl Lznt1Decompress {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.lznt1.decompress@1",
                display_name: "LZNT1 Decompress",
                category: "Compression",
                description: "Decompresses data compressed with LZNT1.",
                cyberchef_alias: Some("LZNT1 Decompress"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Lznt1Decompress {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Lznt1Decompress {
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
        let input = crate::value::take_bytes(input)?;
        Ok(Value::Bytes(codec::decompress(&input, context)?))
    }
}