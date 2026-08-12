use alloc::{collections::BTreeSet, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Ranks portable decode recipes for the input without applying them.
///
/// This is `FerroSift`'s Magic-as-advisor: `CyberChef` `Magic` stays unsupported
/// for interchange because it is a flow-control black box. Use this op when
/// you want deterministic suggestions over the built-in catalog.
pub struct SuggestRecipe {
    spec: OperationSpec,
}

impl SuggestRecipe {
    /// Creates the Suggest recipe operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "analysis.suggest@1",
                display_name: "Suggest recipe",
                category: "Analysis",
                description:
                    "Ranks portable decode recipes for the input without applying them (Magic-as-advisor).",
                cyberchef_alias: None,
                input: ValueConstraint::OneOf(BTreeSet::from([
                    ValueKind::Bytes,
                    ValueKind::Text,
                ])),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument(
                        "depth",
                        "Maximum nested suggestion depth after a successful probe (1-3).",
                        1,
                    ),
                    integer_argument(
                        "max_results",
                        "Maximum number of ranked suggestions to emit (1-32).",
                        8,
                    ),
                    boolean_argument(
                        "intensive",
                        "Enable weaker probes such as ROT13 on alphabetic data.",
                        false,
                    ),
                    text_argument(
                        "crib",
                        "Optional case-insensitive substring that previews/reasons must contain.",
                        "",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for SuggestRecipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for SuggestRecipe {
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
        let report = codec::suggest(
            input,
            integer_value(arguments, "depth")?,
            integer_value(arguments, "max_results")?,
            boolean_value(arguments, "intensive")?,
            text_value(arguments, "crib")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: report,
            encoding: TextEncoding::Utf8,
        }))
    }
}
