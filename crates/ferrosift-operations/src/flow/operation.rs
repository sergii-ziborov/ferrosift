use alloc::{collections::BTreeSet, string::String, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, text_argument};
use crate::spec::{SpecDefinition, build};

/// Marks the start of a mapped multi-branch section (`CyberChef` Fork).
///
/// The executor owns the real map/join semantics. When this operation is
/// invoked on its own it re-joins splits without running a body, which is the
/// empty-body `CyberChef` case.
pub struct Fork {
    spec: OperationSpec,
}

impl Fork {
    /// Creates the Fork flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.fork@1",
                display_name: "Fork",
                category: "Flow control",
                description: "Splits the input and maps subsequent operations onto each branch until Merge.",
                cyberchef_alias: Some("Fork"),
                input: ValueConstraint::OneOf(BTreeSet::from([ValueKind::Text, ValueKind::Bytes])),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "split_delimiter",
                        "Delimiter used to split the input (supports \\n escapes).",
                        "\\n",
                    ),
                    text_argument(
                        "merge_delimiter",
                        "Delimiter used to re-join branch outputs (supports \\n escapes).",
                        "\\n",
                    ),
                    boolean_argument(
                        "ignore_errors",
                        "When true, a failing branch contributes an empty string instead of aborting.",
                        false,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Fork {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Fork {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        // Standalone empty-body behaviour: split and re-join unchanged branches.
        // The executor intercepts recipes that contain a body before Merge.
        context.ensure_active()?;
        let split = crate::jscompat::escape::parse_escaped_chars(crate::args::text_value(
            arguments,
            "split_delimiter",
        )?);
        let merge = crate::jscompat::escape::parse_escaped_chars(crate::args::text_value(
            arguments,
            "merge_delimiter",
        )?);
        let text = value_as_text(input)?;
        let joined = if text.is_empty() {
            String::new()
        } else {
            text.split(split.as_str())
                .collect::<alloc::vec::Vec<_>>()
                .join(merge.as_str())
        };
        if u64::try_from(joined.len()).map_or(true, |size| size > context.budget().max_output_bytes)
        {
            return Err(OperationError::OutputLimitExceeded);
        }
        context.ensure_active()?;
        Ok(Value::Text(TextValue {
            text: joined,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Ends a Fork map region (`CyberChef` Merge).
///
/// Standalone execution is a pure identity: the matching Fork performs the join.
pub struct Merge {
    spec: OperationSpec,
}

impl Merge {
    /// Creates the Merge flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.merge@1",
                display_name: "Merge",
                category: "Flow control",
                description: "Ends a Fork region. Standalone execution is identity.",
                cyberchef_alias: Some("Merge"),
                input: ValueConstraint::Any,
                output: ValueConstraint::Any,
                arguments: vec![boolean_argument(
                    "merge_all",
                    "When true, closes every open Fork/Subsection level.",
                    true,
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Merge {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Merge {
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

fn value_as_text(input: Value) -> Result<alloc::string::String, OperationError> {
    match input {
        Value::Text(text) => Ok(text.text),
        Value::Bytes(bytes) => Ok(match alloc::string::String::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => error.into_bytes().into_iter().map(char::from).collect(),
        }),
        _ => Err(OperationError::InvalidArguments),
    }
}
