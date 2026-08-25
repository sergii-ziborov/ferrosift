use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

fn text_spec(
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
        input: ValueConstraint::Exact(ValueKind::Text),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// Monoalphabetic substitution between two alphabets.
pub struct Substitute {
    spec: OperationSpec,
}

impl Substitute {
    /// Creates the substitution operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "cipher.substitute@1",
                "Substitute",
                "Ciphers",
                "Maps each plaintext symbol to the ciphertext symbol at the same position.",
                "Substitute",
                vec![
                    text_argument(
                        "plaintext",
                        "Source alphabet; range expressions are expanded.",
                        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    ),
                    text_argument(
                        "ciphertext",
                        "Target alphabet; range expressions are expanded.",
                        "XYZABCDEFGHIJKLMNOPQRSTUVW",
                    ),
                    boolean_argument("ignore_case", "Match either case of a symbol.", false),
                ],
            ),
        }
    }
}

impl Default for Substitute {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Substitute {
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
        let plaintext = crate::alphabet::expand(
            text_value(arguments, "plaintext")?,
            "cipher.substitute.invalid_alphabet",
        )?;
        let ciphertext = crate::alphabet::expand(
            text_value(arguments, "ciphertext")?,
            "cipher.substitute.invalid_alphabet",
        )?;
        let ignore_case = boolean_value(arguments, "ignore_case")?;
        let input = take_text(input)?;
        Ok(text_output(codec::substitute(
            &input,
            &plaintext,
            &ciphertext,
            ignore_case,
        )))
    }
}

/// Replaces escape sequences with the characters they name.
pub struct UnescapeString {
    spec: OperationSpec,
}

impl UnescapeString {
    /// Creates the unescaping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.escape.unescape@1",
                "Unescape string",
                "Text",
                "Replaces escape sequences such as \\n and \\x41 with their characters.",
                "Unescape string",
                vec![],
            ),
        }
    }
}

impl Default for UnescapeString {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UnescapeString {
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
        // The same helper the escape-taking arguments already use, so this
        // operation also pins that helper against the reference directly.
        Ok(text_output(crate::escape::parse_escaped_chars(&input)))
    }
}
