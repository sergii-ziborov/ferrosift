use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{bytes as bytes_output, take_text, text as text_output};

use super::codec;

fn spec_for(
    id: &'static str,
    display_name: &'static str,
    category: &'static str,
    description: &'static str,
    alias: &'static str,
    output: ValueConstraint,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category,
        description,
        cyberchef_alias: Some(alias),
        input: ValueConstraint::Exact(ValueKind::Text),
        output,
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// Decodes `^X` and `M-X` sequences.
pub struct CaretMDecode {
    spec: OperationSpec,
}

impl CaretMDecode {
    /// Creates the caret/M decoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "encoding.caret_m.decode@1",
                "Caret/M-decode",
                "Encoding",
                "Decodes caret and M- notation into the bytes they name.",
                "Caret/M-decode",
                ValueConstraint::Exact(ValueKind::Bytes),
                vec![],
            ),
        }
    }
}

impl Default for CaretMDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for CaretMDecode {
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
        let input = take_text(input)?;
        Ok(bytes_output(codec::caret_m_decode(&input, context)?))
    }
}

/// Folds case-insensitive character classes back to letters.
pub struct FromCaseInsensitiveRegex {
    spec: OperationSpec,
}

impl FromCaseInsensitiveRegex {
    /// Creates the folding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "text.regex.case_fold@1",
                "From Case Insensitive Regex",
                "Text",
                "Folds [aA]-style character classes back to a single letter.",
                "From Case Insensitive Regex",
                ValueConstraint::Exact(ValueKind::Text),
                vec![],
            ),
        }
    }
}

impl Default for FromCaseInsensitiveRegex {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromCaseInsensitiveRegex {
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
        let input = take_text(input)?;
        Ok(text_output(codec::from_case_insensitive_regex(&input)))
    }
}

/// Every subset of the delimited items.
pub struct PowerSet {
    spec: OperationSpec,
}

impl PowerSet {
    /// Creates the power-set operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: spec_for(
                "sets.power@1",
                "Power Set",
                "Sets",
                "Lists every subset of the delimited items, one per line.",
                "Power Set",
                ValueConstraint::Exact(ValueKind::Text),
                vec![text_argument(
                    "item_delimiter",
                    "Token separating the items.",
                    ",",
                )],
            ),
        }
    }
}

impl Default for PowerSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for PowerSet {
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
        let delimiter = text_value(arguments, "item_delimiter")?;
        let input = take_text(input)?;
        Ok(text_output(codec::power_set(&input, delimiter, context)?))
    }
}
