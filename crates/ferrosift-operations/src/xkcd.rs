use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::spec::{SpecDefinition, build_generator};

/// Returns 4, the standard IEEE-vetted random number.
///
/// The joke is the operation's whole content — RFC 1149.5, by way of xkcd 221 —
/// and it is a real entry in the reference catalog, so a port that claims to
/// cover the catalog covers this too. It is also the one random-number
/// generator in that catalog whose output can be pinned, which is the reason
/// the others are absent here.
///
/// Its output does not depend on its input, so it is built as a generator: the
/// executor has no expansion ratio to compute from a constant.
///
/// Filed under Text because that is where the catalog's other generator, the
/// De Bruijn sequence, already lives. A one-member Generators family would be
/// the junk drawer the family table exists to prevent.
pub struct XkcdRandomNumber {
    spec: OperationSpec,
}

impl XkcdRandomNumber {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_generator(SpecDefinition {
                id: "text.xkcd_random@1",
                display_name: "XKCD Random Number",
                category: "Text",
                description: "Returns 4, chosen by fair dice roll.",
                cyberchef_alias: Some("XKCD Random Number"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: alloc::vec::Vec::new(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for XkcdRandomNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for XkcdRandomNumber {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        _input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        Ok(Value::Text(TextValue {
            text: alloc::string::String::from("4"),
            encoding: TextEncoding::Utf8,
        }))
    }
}
