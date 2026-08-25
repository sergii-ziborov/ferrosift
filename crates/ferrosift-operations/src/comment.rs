use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint};

use crate::args::text_argument;
use crate::spec::{SpecDefinition, build};

/// Carries a note through a recipe without touching the value.
///
/// This is not a duplicate of `Identity`. Identity is the do-nothing operation
/// a caller reaches for when a step is required and none is wanted; a comment
/// is a place to write down *why* the surrounding steps are what they are, and
/// it holds that text in an argument the recipe carries with it. Collapsing the
/// two would either lose the note or give Identity a field it has no use for.
///
/// The reference marks this as flow control, but it returns the state it was
/// given, so there is nothing for the executor to special-case.
pub struct Comment {
    spec: OperationSpec,
}

impl Comment {
    /// Creates the comment operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.comment@1",
                display_name: "Comment",
                category: "Flow control",
                description: "Holds a note in the recipe and passes the value through unchanged.",
                cyberchef_alias: Some("Comment"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![text_argument("comment", "Text to record in the recipe.", "")],
                inverse: Some("flow.comment@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for Comment {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Comment {
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
