use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint};

use crate::args::text_argument;
use crate::spec::{SpecDefinition, build};

/// Names a place in a recipe, and passes the value through untouched.
///
/// A marker rather than a transformation: the reference's own body is one
/// line that returns the state it was given. Like [`crate::Comment`], it is
/// marked flow control there and needs nothing from the executor here.
///
/// What it names is a jump target, and `FerroSift` has no Jump. That does not
/// make the operation incomplete: a recipe carrying a Label behaves exactly as
/// the reference's does, because a Label with nothing jumping to it is a
/// pass-through there too. What is missing is Jump, which is listed as missing
/// -- and which would need a program counter the linear executor does not
/// have, rather than another operation.
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
