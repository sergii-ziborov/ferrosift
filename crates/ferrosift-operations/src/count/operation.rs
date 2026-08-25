use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};
use regex_automata::{meta::Regex, util::syntax};

use crate::args::{map_argument, map_value, toggle_string_default, toggle_string_parts};
use crate::jscompat::escape::parse_escaped_chars;
use crate::spec::{SpecDefinition, build};
use crate::value::take_text;

/// Counts occurrences of a token or pattern.
pub struct CountOccurrences {
    spec: OperationSpec,
}

impl CountOccurrences {
    /// Creates the counting operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "text.count@1",
                display_name: "Count occurrences",
                category: "Text",
                description: "Counts how many times a token or pattern appears in the input.",
                cyberchef_alias: Some("Count occurrences"),
                input: ValueConstraint::Exact(ValueKind::Text),
                // A count, not a rendering of one. Emitting text would make
                // the caller parse a number back out to use it.
                output: ValueConstraint::Exact(ValueKind::Integer),
                // One toggleString in the reference, not two arguments: the
                // mode and the token travel together, and recipe import maps
                // them positionally as a pair.
                arguments: vec![map_argument(
                    "search_string",
                    "Token or pattern to count, with the mode that reads it.",
                    toggle_string_default("Simple string", ""),
                )],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for CountOccurrences {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for CountOccurrences {
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
        let (search_type, search) = toggle_string_parts(map_value(arguments, "search_string")?)?;
        let input = take_text(input)?;

        // An empty search counts nothing. The reference guards on the string
        // being truthy before either branch, so this comes first.
        if search.is_empty() {
            return Ok(Value::Integer(0));
        }

        let count = match search_type {
            "Regex" => regex_count(&input, search),
            value if value.starts_with("Extended") => {
                literal_count(&input, &parse_escaped_chars(search))
            }
            _ => literal_count(&input, search),
        };
        context.ensure_active()?;
        Ok(Value::Integer(i128::from(count)))
    }
}

/// Non-overlapping literal occurrences.
///
/// The reference counts with `split(needle).length - 1`, which is
/// non-overlapping and counts left to right — the same thing `matches` does.
fn literal_count(input: &str, needle: &str) -> u32 {
    if needle.is_empty() {
        return 0;
    }
    u32::try_from(input.matches(needle).count()).unwrap_or(u32::MAX)
}

/// Pattern occurrences, case-insensitively.
///
/// The reference builds its expression with the `gi` flags and wraps the whole
/// thing in a `try`: a pattern that will not compile, and a pattern that
/// matches nothing, both come back as zero rather than as an error. Returning
/// zero for an unparsable pattern is not obviously right, but it is what the
/// reference does and what a recipe carrying one will expect.
fn regex_count(input: &str, pattern: &str) -> u32 {
    let Ok(regex) = Regex::builder()
        .syntax(
            syntax::Config::new()
                .case_insensitive(true)
                .unicode(true)
                .utf8(true),
        )
        .build(pattern)
    else {
        return 0;
    };
    u32::try_from(regex.find_iter(input.as_bytes()).count()).unwrap_or(u32::MAX)
}
