use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint};

use crate::args::text_argument;
use crate::spec::{SpecDefinition, build};

/// Names a place in a recipe, and passes the value through untouched.
///
/// A marker rather than a transformation: the reference's own body is one
/// line that returns the state it was given. Like [`crate::Comment`], it is
/// marked flow control there and produces nothing the executor has to act on.
///
/// What it names is a destination. [`crate::Jump`] and [`crate::ConditionalJump`]
/// find it by the name in this argument, and the executor resumes at the step
/// *after* it — so a Label with nothing jumping to it is still exactly a
/// pass-through, which is what it was before either of them existed.
///
/// A disabled Label is still a destination. The reference's lookup does not ask
/// whether the step is enabled, and landing on a step that is skipped changes
/// nothing anyway.
pub struct Label {
    spec: OperationSpec,
}

impl Label {
    /// Creates the label operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.label@1",
                display_name: "Label",
                category: "Flow control",
                description: "Names a position in the recipe and passes the value through.",
                cyberchef_alias: Some("Label"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![text_argument("name", "The label's name.", "")],
                inverse: Some("flow.label@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for Label {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Label {
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
