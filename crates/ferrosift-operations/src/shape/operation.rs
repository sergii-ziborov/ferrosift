use alloc::string::ToString;
use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
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

/// Removes ANSI control sequences.
pub struct RemoveAnsiEscapeCodes {
    spec: OperationSpec,
}

impl RemoveAnsiEscapeCodes {
    /// Creates the ANSI-stripping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.ansi.strip@1",
                "Remove ANSI Escape Codes",
                "Removes ANSI terminal control sequences from the input.",
                "Remove ANSI Escape Codes",
                vec![],
            ),
        }
    }
}

impl Default for RemoveAnsiEscapeCodes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RemoveAnsiEscapeCodes {
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
        Ok(text_output(codec::remove_ansi(&input)))
    }
}

/// Drops HTTP headers, keeping the body.
pub struct StripHttpHeaders {
    spec: OperationSpec,
}

impl StripHttpHeaders {
    /// Creates the header-stripping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "http.headers.strip@1",
                "Strip HTTP headers",
                "Removes everything up to and including the first blank line.",
                "Strip HTTP headers",
                vec![],
            ),
        }
    }
}

impl Default for StripHttpHeaders {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for StripHttpHeaders {
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
        Ok(text_output(codec::strip_http_headers(&input).to_string()))
    }
}

/// Reassembles a chunked HTTP body.
pub struct DechunkHttpResponse {
    spec: OperationSpec,
}

impl DechunkHttpResponse {
    /// Creates the dechunking operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "http.dechunk@1",
                "Dechunk HTTP response",
                "Reassembles a chunked transfer-encoded HTTP body.",
                "Dechunk HTTP response",
                vec![],
            ),
        }
    }
}

impl Default for DechunkHttpResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DechunkHttpResponse {
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
        Ok(text_output(codec::dechunk(&input, context)?))
    }
}

/// Breaks the input into fixed-width lines.
pub struct Wrap {
    spec: OperationSpec,
}

impl Wrap {
    /// Creates the wrapping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.wrap@1",
                "Wrap",
                "Breaks the input into lines of at most the given width.",
                "Wrap",
                vec![integer_argument(
                    "line_width",
                    "Maximum characters per line.",
                    64,
                )],
            ),
        }
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Wrap {
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
        // The reference declares the width as an integer between 1 and 65536.
        // A width of zero would build the regex `.{1,0}`, which matches
        // nothing and would loop rather than wrap, so it is refused here
        // instead of being discovered at runtime.
        let width = integer_value(arguments, "line_width")?;
        let width = usize::try_from(width)
            .ok()
            .filter(|value| (1..=65536).contains(value))
            .ok_or(OperationError::InvalidArguments)?;
        let input = take_text(input)?;
        Ok(text_output(codec::wrap(&input, width)))
    }
}

/// Expands an alphabet range expression into its characters.
pub struct ExpandAlphabetRange {
    spec: OperationSpec,
}

impl ExpandAlphabetRange {
    /// Creates the alphabet-range operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "text.alphabet.expand@1",
                "Expand alphabet range",
                "Expands a range expression such as a-z into its characters.",
                "Expand alphabet range",
                vec![text_argument(
                    "delimiter",
                    "Delimiter placed between expanded characters.",
                    "",
                )],
            ),
        }
    }
}

impl Default for ExpandAlphabetRange {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExpandAlphabetRange {
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
        let delimiter = text_value(arguments, "delimiter")?;
        let input = take_text(input)?;
        // The same expansion the alphabet-taking operations already use, so
        // this operation is also a direct check on that shared helper against
        // the reference.
        let expanded = crate::alphabet::expand(&input, "text.alphabet.invalid_range")?;
        let mut output = alloc::string::String::new();
        for (index, value) in expanded.iter().enumerate() {
            if index > 0 {
                output.push_str(delimiter);
            }
            output.push(*value);
        }
        Ok(text_output(output))
    }
}
