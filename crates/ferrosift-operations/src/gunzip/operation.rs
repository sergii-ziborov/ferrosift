use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Decompresses gzip-wrapped deflate data.
pub struct Gunzip {
    spec: OperationSpec,
}

impl Gunzip {
    /// Creates the gunzip operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.gunzip@1",
                display_name: "Gunzip",
                category: "Compression",
                description: "Decompresses gzip-wrapped deflate data.",
                cyberchef_alias: Some("Gunzip"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: None,
            }),
        }
    }
}

impl Default for Gunzip {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Gunzip {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::decompress(&input, context)?))
    }
}
