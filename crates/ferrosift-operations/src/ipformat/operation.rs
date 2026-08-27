use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

const FORMATS: &str = "Dotted Decimal, Decimal, Octal, or Hex";

/// One IPv4 address, rewritten in another notation.
pub struct ChangeIpFormat {
    spec: OperationSpec,
}

impl ChangeIpFormat {
    /// Creates the address-format conversion.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "network.ip.format@1",
                display_name: "Change IP format",
                category: "Networking",
                description: "Rewrites each IPv4 address in the input in another notation.",
                cyberchef_alias: Some("Change IP format"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "input_format",
                        &alloc::format!("How the input is written: {FORMATS}."),
                        "Dotted Decimal",
                    ),
                    text_argument(
                        "output_format",
                        &alloc::format!("How to write the answer: {FORMATS}."),
                        "Decimal",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ChangeIpFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ChangeIpFormat {
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
        let from = codec::format(text_value(arguments, "input_format")?)
            .ok_or_else(|| failed("network.ip.format.unknown_input"))?;
        let to = codec::format(text_value(arguments, "output_format")?)
            .ok_or_else(|| failed("network.ip.format.unknown_output"))?;
        let input = take_text(input)?;

        let mut lines: vec::Vec<String> = vec::Vec::new();
        for line in input.split('\n') {
            // An empty line contributes nothing, not even a blank -- so the
            // answer has fewer lines than the input where the input had gaps.
            if line.is_empty() {
                continue;
            }
            // Asked for the notation it already has, a line is copied through
            // without being read. That is not the same as a round trip: a
            // malformed line survives here and would not survive being parsed.
            if from == to {
                lines.push(String::from(line));
                continue;
            }
            // Nothing here refuses: the reference reads whatever it is given
            // and writes whatever came back, so a malformed line produces a
            // malformed answer rather than an error.
            lines.push(codec::write(&codec::read(line, from), to));
        }

        context.ensure_active()?;
        Ok(text_output(lines.join("\n")))
    }
}
