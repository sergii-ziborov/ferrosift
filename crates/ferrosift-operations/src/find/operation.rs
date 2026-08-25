use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{
    boolean_argument, boolean_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Finds and replaces substrings or regular expressions.
pub struct FindReplace {
    spec: OperationSpec,
}

impl FindReplace {
    /// Creates the find/replace operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "text.find_replace@1",
                display_name: "Find / Replace",
                category: "Text",
                description: "Replaces all occurrences of a pattern in the input text.",
                cyberchef_alias: Some("Find / Replace"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    map_argument(
                        "find",
                        "Pattern and mode (Regex, Extended, Simple string).",
                        toggle_string_default("Regex", ""),
                    ),
                    text_argument(
                        "replace",
                        "Replacement text; supports escape sequences.",
                        "",
                    ),
                    boolean_argument("global_match", "Replace every match.", true),
                    boolean_argument("case_insensitive", "Match without regard to case.", false),
                    boolean_argument("multiline_matching", "Enable multiline anchors.", true),
                    boolean_argument("dot_matches_all", "Allow '.' to match newlines.", false),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for FindReplace {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FindReplace {
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
        let input = crate::value::take_text_value(input)?;
        let (option, string) = toggle_string_parts(map_value(arguments, "find")?)?;
        let mut flag_bits = 0_u8;
        if boolean_value(arguments, "global_match")? {
            flag_bits |= codec::MatchFlags::GLOBAL;
        }
        if boolean_value(arguments, "case_insensitive")? {
            flag_bits |= codec::MatchFlags::CASE_INSENSITIVE;
        }
        if boolean_value(arguments, "multiline_matching")? {
            flag_bits |= codec::MatchFlags::MULTILINE;
        }
        if boolean_value(arguments, "dot_matches_all")? {
            flag_bits |= codec::MatchFlags::DOT_ALL;
        }
        let output = codec::replace(
            &input.text,
            option,
            string,
            text_value(arguments, "replace")?,
            codec::MatchFlags::from_bits(flag_bits),
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}
