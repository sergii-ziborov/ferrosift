use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

/// The representations a pass-through accepts and returns unchanged.
///
/// Deliberately wide: a comment placed between two byte operations is exactly
/// where someone puts one, and constraining to text would refuse it.
fn passthrough_kinds() -> alloc::collections::BTreeSet<ValueKind> {
    [ValueKind::Bytes, ValueKind::Text, ValueKind::Empty]
        .into_iter()
        .collect()
}

fn passthrough_spec(
    id: &'static str,
    display_name: &'static str,
    category: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category,
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::OneOf(passthrough_kinds()),
        output: ValueConstraint::OneOf(passthrough_kinds()),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// Renders HTML as text.
pub struct HtmlToText {
    spec: OperationSpec,
}

impl HtmlToText {
    /// Creates the HTML-to-text operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: passthrough_spec(
                "text.html.to_text@1",
                "HTML To Text",
                "Text",
                "Returns its input unchanged; the reference renders it in a presentation layer.",
                "HTML To Text",
                vec![],
            ),
        }
    }
}

impl Default for HtmlToText {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for HtmlToText {
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
        // The reference's `run` is `return input`. Its work happens in
        // `present`, which draws the HTML in a browser — a stage FerroSift
        // does not have and does not claim to. The data transformation is
        // therefore genuinely the identity, and saying so is more honest than
        // inventing a text extraction the reference never performs.
        Ok(input)
    }
}
