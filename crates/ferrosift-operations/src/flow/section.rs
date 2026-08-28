//! Scoping the rest of a recipe to the parts of a value that match a pattern.
//!
//! The operation finds the spans; the executor runs the region on each of them
//! and puts the pieces back. Splitting the work that way is what keeps the
//! regular expression engine in this crate, behind the `text` feature, and out
//! of a `no_std` core that has no business compiling a pattern.

use alloc::{string::String, vec, vec::Vec};

use ferrosift_core::{FlowDirective, Operation, OperationContext, OperationError, Section};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};
use regex_automata::{meta::Regex, util::syntax};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};

const INVALID_PATTERN: &str = "flow.section.invalid_pattern";

/// Runs the rest of the recipe on each part of the value a pattern selects.
///
/// The region it opens is closed by `Merge`, is nested the same way a `Fork`
/// is, and differs from a Fork in one respect that decides everything else:
/// what lies *between* the selected parts is carried through untouched rather
/// than dropped and re-joined.
///
/// With a capture group the region runs on the group rather than on the whole
/// match, which is how a pattern says "find it by its surroundings, change only
/// this". Only the first group is used; the reference says the same and means
/// it slightly differently — see `docs/compatibility/cyberchef-v11.3.0.md`.
pub struct Subsection {
    spec: OperationSpec,
}

impl Subsection {
    /// Creates the Subsection flow-control operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "flow.subsection@1",
                display_name: "Subsection",
                category: "Flow control",
                description: "Runs subsequent operations on each part of the input matching a regular expression, until Merge.",
                cyberchef_alias: Some("Subsection"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("pattern", "Regular expression selecting the sections.", ""),
                    boolean_argument("case_sensitive", "Match case sensitively.", true),
                    boolean_argument("global", "Select every match rather than the first.", true),
                    boolean_argument(
                        "ignore_errors",
                        "When true, a failing section contributes nothing instead of aborting.",
                        false,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Subsection {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Subsection {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        // Standalone: a subsection with nothing after it selects parts of the
        // value and runs an empty recipe on each, which puts the value back
        // exactly as it was. The executor intercepts every recipe that has a
        // body, so this is the empty case rather than a shortcut past one.
        context.ensure_active()?;
        Ok(Value::Text(TextValue {
            text: as_text(&input),
            encoding: TextEncoding::Utf8,
        }))
    }

    fn direct(
        &self,
        value: &Value,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<FlowDirective, OperationError> {
        context.ensure_active()?;
        let pattern = text_value(arguments, "pattern")?;
        let subject = as_text(value);
        if pattern.is_empty() || subject.is_empty() {
            // Nothing to scope. The reference returns its state untouched and
            // the following operations run on the whole value, so this is a
            // fall-through rather than a region with no sections in it.
            return Ok(FlowDirective::Next);
        }
        let regex = compile(pattern, boolean_value(arguments, "case_sensitive")?)?;
        let global = boolean_value(arguments, "global")?;
        // One more than the executor will accept, so an oversized selection is
        // refused by the budget that owns that question instead of being
        // silently truncated here.
        let ceiling = context.budget().max_branches.saturating_add(1);
        let mut spans = Vec::new();
        for captures in regex.captures_iter(subject.as_bytes()) {
            let Some(whole) = captures.get_group(0) else {
                break;
            };
            // A capture group narrows the section to the group. A pattern with
            // groups whose first one did not participate has nothing narrower
            // to offer, so the whole match stands.
            let chosen = if captures.group_len() > 1 {
                captures.get_group(1).unwrap_or(whole)
            } else {
                whole
            };
            spans.push(Section::new(chosen.start, chosen.end));
            if !global || spans.len() >= ceiling {
                break;
            }
            if spans.len().is_multiple_of(256) {
                context.ensure_active()?;
            }
        }
        Ok(FlowDirective::Sections {
            spans,
            ignore_errors: boolean_value(arguments, "ignore_errors")?,
        })
    }
}

/// Whether a pattern matches anywhere in the subject.
///
/// `Conditional Jump` asks this and nothing else. The reference asks it with
/// `String.prototype.search`, which compiles the pattern with no flags at all.
pub(super) fn matches(pattern: &str, subject: &str) -> Result<bool, OperationError> {
    Ok(compile(pattern, true)?.is_match(subject.as_bytes()))
}

/// The reference's string view of a dish, without consuming the value.
///
/// Text as it stands; bytes as one character per byte when they are not valid
/// UTF-8, which is what `Utils.byteArrayToChars` does; anything else through
/// the model's own conversion, which is the projection a later step would see.
pub(super) fn as_text(value: &Value) -> String {
    match value {
        Value::Text(text) => text.text.clone(),
        Value::Bytes(bytes) => match core::str::from_utf8(bytes) {
            Ok(text) => String::from(text),
            Err(_) => bytes.iter().copied().map(char::from).collect(),
        },
        other => match other.clone().reinterpret(ValueKind::Text) {
            Some(Value::Text(text)) => text.text,
            _ => String::new(),
        },
    }
}

fn compile(pattern: &str, case_sensitive: bool) -> Result<Regex, OperationError> {
    Regex::builder()
        .syntax(
            syntax::Config::new()
                .case_insensitive(!case_sensitive)
                .unicode(true)
                .utf8(true),
        )
        .build(pattern)
        .map_err(|_| failed(INVALID_PATTERN))
}

#[cfg(test)]
mod tests {
    use super::{as_text, matches};
    use alloc::vec;
    use ferrosift_model::{TextEncoding, TextValue, Value};

    #[test]
    fn bytes_are_read_one_character_per_byte_when_not_utf8() {
        assert_eq!(as_text(&Value::Bytes(vec![0xff, 0x41])), "\u{ff}A");
        assert_eq!(as_text(&Value::Bytes(vec![0xc3, 0xa9])), "é");
    }

    #[test]
    fn a_condition_is_a_search_and_not_an_anchored_match() {
        let value = Value::Text(TextValue {
            text: alloc::string::String::from("hello"),
            encoding: TextEncoding::Utf8,
        });
        assert!(matches("ell", &as_text(&value)).expect("valid pattern"));
        assert!(!matches("^ell", &as_text(&value)).expect("valid pattern"));
        assert!(matches("l+o$", &as_text(&value)).expect("valid pattern"));
    }
}
