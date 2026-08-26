use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    ArgumentSpec, Arguments, DecimalValue, OperationSpec, Value, ValueConstraint, ValueKind,
};

use crate::args::{text_argument, text_value};
use crate::failure::failed;
use crate::jscompat::bignumber;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec::{self, Unit};

/// The unit names the reference knows, for an argument's description.
///
/// Spelled out rather than summarised because the micro sign is U+03BC, the
/// Greek letter, and not the visually identical U+00B5 -- so a caller writing
/// the argument by hand has the right one in front of them.
const UNITS: &str = "Seconds (s), Milliseconds (ms), Microseconds (\u{03bc}s), or Nanoseconds (ns)";
const FORMATS: &str = "Decimal, Hex (big endian), or Hex (little endian)";

/// The units argument, whose meaning differs between the two directions.
fn units_argument(role: &str) -> ArgumentSpec {
    text_argument("units", &alloc::format!("{role}: {UNITS}."), "Seconds (s)")
}

/// The format argument, whose meaning differs between the two directions.
fn format_argument(role: &str) -> ArgumentSpec {
    text_argument("format", &alloc::format!("{role}: {FORMATS}."), "Decimal")
}

/// A UNIX timestamp as a Windows filetime.
pub struct UnixToFiletime {
    spec: OperationSpec,
}

impl UnixToFiletime {
    /// Creates the UNIX-to-filetime operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "time.filetime.encode@1",
                display_name: "UNIX Timestamp to Windows Filetime",
                category: "Time",
                description: "Converts a UNIX timestamp to the hundred-nanosecond intervals since 1601 that a Windows filetime counts.",
                cyberchef_alias: Some("UNIX Timestamp to Windows Filetime"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    units_argument("The units the input is in"),
                    format_argument("How to write the answer"),
                ],
                inverse: Some("time.filetime.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for UnixToFiletime {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UnixToFiletime {
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
        let units = text_value(arguments, "units")?;
        let format = text_value(arguments, "format")?;
        let input = take_text(input)?;

        // Empty input answers an empty string rather than a refusal, which is
        // the reference's own guard and not a courtesy added here.
        if input.is_empty() {
            return Ok(text_output(String::new()));
        }

        // The reference calls the constructor directly here, so text it cannot
        // read stops the recipe -- unlike a dish, which would substitute
        // not-a-number and carry on.
        let value = DecimalValue::read(&input).ok_or_else(|| failed("time.filetime.unreadable"))?;
        let unit = codec::unit(units).ok_or_else(|| failed("time.filetime.unknown_units"))?;
        let scaled = match unit {
            Unit::Times(factor) => bignumber::times(&value, &codec::factor(factor)),
            Unit::Over(factor) => bignumber::divide(&value, &codec::factor(factor)),
        };
        let shifted = bignumber::plus(&scaled, &codec::factor(codec::EPOCH_OFFSET));

        context.ensure_active()?;
        // Anything not beginning with `Hex` is decimal. That is the
        // reference's test, not a listing of the three names its interface
        // offers, so a fourth name behaves as decimal rather than failing.
        let written = if format.starts_with("Hex") {
            bignumber::to_base(&shifted, 16).ok_or_else(|| failed("time.filetime.unwritable"))?
        } else {
            shifted.to_fixed()
        };
        let written = if format == "Hex (little endian)" {
            codec::flip_forward(&written)
        } else {
            written
        };
        Ok(text_output(written))
    }
}

/// A Windows filetime as a UNIX timestamp.
pub struct FiletimeToUnix {
    spec: OperationSpec,
}

impl FiletimeToUnix {
    /// Creates the filetime-to-UNIX operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "time.filetime.decode@1",
                display_name: "Windows Filetime to UNIX Timestamp",
                category: "Time",
                description: "Converts a Windows filetime, in hundred-nanosecond intervals since 1601, to a UNIX timestamp.",
                cyberchef_alias: Some("Windows Filetime to UNIX Timestamp"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    units_argument("The units to answer in"),
                    format_argument("How the input is written"),
                ],
                inverse: Some("time.filetime.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FiletimeToUnix {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FiletimeToUnix {
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
        let units = text_value(arguments, "units")?;
        let format = text_value(arguments, "format")?;
        let input = take_text(input)?;

        if input.is_empty() {
            return Ok(text_output(String::new()));
        }

        // The bytes are reordered before they are read, not after, so an
        // odd-length input is rearranged by the rule in the codec rather than
        // padded into an even one.
        let reordered = if format == "Hex (little endian)" {
            codec::flip_back(&input)
        } else {
            input
        };
        let value = if format.starts_with("Hex") {
            bignumber::parse_in_base(&reordered, 16)
                .ok_or_else(|| failed("time.filetime.unreadable"))?
        } else {
            DecimalValue::read(&reordered).ok_or_else(|| failed("time.filetime.unreadable"))?
        };

        // The epoch comes off before the units go on in this direction, which
        // is the reverse of the other one -- and the reverse of what a reader
        // would guess from the name.
        let shifted = bignumber::minus(&value, &codec::factor(codec::EPOCH_OFFSET));
        let unit = codec::unit(units).ok_or_else(|| failed("time.filetime.unknown_units"))?;

        context.ensure_active()?;
        let scaled = match unit {
            Unit::Times(factor) => bignumber::divide(&shifted, &codec::factor(factor)),
            Unit::Over(factor) => bignumber::times(&shifted, &codec::factor(factor)),
        };
        Ok(text_output(scaled.to_fixed()))
    }
}
