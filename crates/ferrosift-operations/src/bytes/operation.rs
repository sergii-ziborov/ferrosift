use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, integer_argument, integer_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Takes a byte slice from the input.
pub struct TakeBytes {
    spec: OperationSpec,
}

impl TakeBytes {
    /// Creates the take-bytes operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "data.take_bytes@1",
                display_name: "Take bytes",
                category: "Data",
                description: "Takes a slice of the specified number of bytes from the data.",
                cyberchef_alias: Some("Take bytes"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    integer_argument(
                        "start",
                        "Start offset; negative values count from the end.",
                        0,
                    ),
                    integer_argument(
                        "length",
                        "Number of bytes to take; negative flips the window.",
                        5,
                    ),
                    boolean_argument(
                        "apply_to_each_line",
                        "Apply the slice independently to each LF-delimited line.",
                        false,
                    ),
                ],
                inverse: None,
            }),
        }
    }
}

impl Default for TakeBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for TakeBytes {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::take(
            &input,
            integer_value(arguments, "start")?,
            integer_value(arguments, "length")?,
            boolean_value(arguments, "apply_to_each_line")?,
            context,
        )?))
    }
}

/// Drops a byte slice from the input.
pub struct DropBytes {
    spec: OperationSpec,
}

impl DropBytes {
    /// Creates the drop-bytes operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "data.drop_bytes@1",
                display_name: "Drop bytes",
                category: "Data",
                description: "Cuts a slice of the specified number of bytes out of the data.",
                cyberchef_alias: Some("Drop bytes"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    integer_argument(
                        "start",
                        "Start offset; negative values count from the end.",
                        0,
                    ),
                    integer_argument(
                        "length",
                        "Number of bytes to drop; negative flips the window.",
                        5,
                    ),
                    boolean_argument(
                        "apply_to_each_line",
                        "Apply the cut independently to each LF-delimited line.",
                        false,
                    ),
                ],
                inverse: None,
            }),
        }
    }
}

impl Default for DropBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DropBytes {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::drop(
            &input,
            integer_value(arguments, "start")?,
            integer_value(arguments, "length")?,
            boolean_value(arguments, "apply_to_each_line")?,
            context,
        )?))
    }
}
