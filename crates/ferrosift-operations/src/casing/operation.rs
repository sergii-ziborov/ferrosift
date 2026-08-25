use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{take_text, text as text_output};

use super::codec;

/// Builds a text-in / text-out specification for this family.
fn text_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Text,
        UniformSpec {
            id,
            display_name,
            category: "Text",
            description,
            cyberchef_alias: alias,
            arguments,
        },
    )
}

/// Lower-cases the whole input.
pub struct ToLowerCase {
    spec: OperationSpec,
}

impl ToLowerCase {
    /// Creates the lower-case operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.case.lower@1",
                "To Lower case",
                "Converts the input to lower case.",
                "To Lower case",
                vec![],
            ),
        }
    }
}

impl Default for ToLowerCase {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToLowerCase {
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
        Ok(text_output(codec::lower(&input)))
    }
}

/// Upper-cases the input, over a chosen scope.
pub struct ToUpperCase {
    spec: OperationSpec,
}

impl ToUpperCase {
    /// Creates the upper-case operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.case.upper@1",
                "To Upper case",
                "Converts the input to upper case over the chosen scope.",
                "To Upper case",
                vec![text_argument(
                    "scope",
                    "Capitalisation scope: All, Word, Sentence, Paragraph.",
                    "All",
                )],
            ),
        }
    }
}

impl Default for ToUpperCase {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToUpperCase {
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
        let scope = codec::scope(text_value(arguments, "scope")?)?;
        let input = take_text(input)?;
        Ok(text_output(codec::capitalise(&input, scope)))
    }
}

/// Swaps the case of every character.
pub struct SwapCase {
    spec: OperationSpec,
}

impl SwapCase {
    /// Creates the swap-case operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.case.swap@1",
                "Swap case",
                "Swaps the case of every character in the input.",
                "Swap case",
                vec![],
            ),
        }
    }
}

impl Default for SwapCase {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for SwapCase {
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
        Ok(text_output(codec::swap_case(&input)))
    }
}

/// Alternates case across the letters of the input.
pub struct AlternatingCaps {
    spec: OperationSpec,
}

impl AlternatingCaps {
    /// Creates the alternating-caps operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.case.alternating@1",
                "Alternating Caps",
                "Alternates case across letters, leaving other characters alone.",
                "Alternating Caps",
                vec![],
            ),
        }
    }
}

impl Default for AlternatingCaps {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for AlternatingCaps {
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
        Ok(text_output(codec::alternating(&input)))
    }
}

/// Every combination of upper and lower case, one per line.
pub struct GetAllCasings {
    spec: OperationSpec,
}

impl GetAllCasings {
    /// Creates the all-casings operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.case.all@1",
                "Get All Casings",
                "Lists every combination of upper and lower case, one per line.",
                "Get All Casings",
                vec![],
            ),
        }
    }
}

impl Default for GetAllCasings {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for GetAllCasings {
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
        Ok(text_output(codec::all_casings(&input, context)?))
    }
}
