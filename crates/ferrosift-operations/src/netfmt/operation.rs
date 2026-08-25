use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec::{self, MacStyles};

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

/// Adds or removes a parity bit on runs of binary digits.
pub struct ParityBit {
    spec: OperationSpec,
}

impl ParityBit {
    /// Creates the parity-bit operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "logic.parity@1",
                "Parity Bit",
                "Logic",
                "Adds or removes a parity bit on runs of binary digits.",
                "Parity Bit",
                vec![
                    text_argument("mode", "Even Parity or Odd Parity.", "Even Parity"),
                    text_argument(
                        "postion",
                        "Which end the bit sits on: Start or End.",
                        "Start",
                    ),
                    text_argument("encode_or_decode", "Encode or Decode.", "Encode"),
                    text_argument(
                        "delimiter",
                        "Field separator; empty treats the input as one run.",
                        "",
                    ),
                ],
            ),
        }
    }
}

impl Default for ParityBit {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ParityBit {
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
        let parity = codec::parity(text_value(arguments, "mode")?)?;
        // The argument is spelled "Postion" upstream. The alias has to match
        // the reference exactly for recipe import to work, so the typo is
        // carried rather than quietly corrected.
        let position = codec::position(text_value(arguments, "postion")?)?;
        let direction = codec::direction(text_value(arguments, "encode_or_decode")?)?;
        let delimiter = text_value(arguments, "delimiter")?;
        let input = take_text(input)?;
        Ok(text_output(codec::parity_bit(
            &input, parity, position, direction, delimiter,
        )?))
    }
}

/// Rewrites MAC addresses in every requested delimiter style.
pub struct FormatMacAddresses {
    spec: OperationSpec,
}

impl FormatMacAddresses {
    /// Creates the MAC formatting operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "network.mac.format@1",
                "Format MAC addresses",
                "Networking",
                "Rewrites each MAC address in the requested delimiter styles and cases.",
                "Format MAC addresses",
                vec![
                    text_argument("output_case", "Both, Upper only, or Lower only.", "Both"),
                    boolean_argument("no_delimiter", "Emit the address with no separator.", true),
                    boolean_argument("dash_delimiter", "Emit octets separated by dashes.", true),
                    boolean_argument("colon_delimiter", "Emit octets separated by colons.", true),
                    boolean_argument(
                        "cisco_style",
                        "Emit four-digit groups separated by dots.",
                        false,
                    ),
                    boolean_argument(
                        "ipv6_interface_id",
                        "Emit the EUI-64 interface identifier.",
                        false,
                    ),
                ],
            ),
        }
    }
}

impl Default for FormatMacAddresses {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FormatMacAddresses {
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
        let case = codec::output_case(text_value(arguments, "output_case")?)?;
        let styles = MacStyles {
            none: boolean_value(arguments, "no_delimiter")?,
            dash: boolean_value(arguments, "dash_delimiter")?,
            colon: boolean_value(arguments, "colon_delimiter")?,
            cisco: boolean_value(arguments, "cisco_style")?,
            ipv6: boolean_value(arguments, "ipv6_interface_id")?,
        };
        let input = take_text(input)?;
        Ok(text_output(codec::format_macs(&input, case, styles)))
    }
}
