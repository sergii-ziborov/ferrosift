use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint};

use crate::spec::{SpecDefinition, build};

/// Preserves a value and its representation unchanged.
pub struct Identity {
    spec: OperationSpec,
}

impl Identity {
    /// Creates the identity operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "core.identity@1",
                display_name: "Identity",
                category: "Core",
                description: "Preserves the input value and representation unchanged.",
                cyberchef_alias: None,
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: alloc::vec::Vec::new(),
                inverse: Some("core.identity@1"),
            }),
        }
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Identity {
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
        Ok(input)
    }
}
