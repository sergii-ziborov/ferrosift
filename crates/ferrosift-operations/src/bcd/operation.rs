use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    ArgumentSpec, Arguments, DecimalValue, OperationSpec, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_bytes, take_decimal, take_text, text as text_output};

use super::codec;

const SCHEMES: &str = "8 4 2 1, 7 4 2 1, 4 2 2 1, 2 4 2 1, 8 4 -2 -1, Excess-3, or IBM 8 4 2 1";

/// The four arguments both directions carry, in the reference's order.
fn arguments(format_role: &str) -> vec::Vec<ArgumentSpec> {
    vec![
        text_argument(
            "scheme",
            &alloc::format!("Which nibble each digit takes: {SCHEMES}."),
            "8 4 2 1",
        ),
        boolean_argument("packed", "Whether two nibbles share a byte.", true),
        boolean_argument("signed", "Whether a sign nibble is appended.", false),
        text_argument(
            "format",
            &alloc::format!("{format_role}: Nibbles, Bytes, or Raw."),
            "Nibbles",
        ),
    ]
}

/// Reads the four arguments, refusing a scheme the reference has no table for.
fn settings<'a>(
    arguments: &'a Arguments,
    unknown_scheme: &'static str,
) -> Result<([u8; 10], bool, bool, &'a str), OperationError> {
    let name = text_value(arguments, "scheme")?;
    let table = codec::scheme(name).ok_or_else(|| failed(unknown_scheme))?;
    Ok((
        table,
        boolean_value(arguments, "packed")?,
        boolean_value(arguments, "signed")?,
        text_value(arguments, "format")?,
    ))
}

/// A number as binary-coded decimal.
pub struct ToBcd {
    spec: OperationSpec,
}

impl ToBcd {
    /// Creates the To BCD operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.bcd.encode@1",
                display_name: "To BCD",
                category: "Encoding",
                description: "Writes a whole number with one nibble per decimal digit.",
                cyberchef_alias: Some("To BCD"),
                input: ValueConstraint::Exact(ValueKind::Decimal),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: arguments("How to write the answer"),
                inverse: Some("encoding.bcd.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBcd {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBcd {
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
        let (table, packed, signed, format) =
            settings(arguments, "encoding.bcd.encode.unknown_scheme")?;
        let value = take_decimal(input)?;

        if value.is_not_a_number() {
            return Err(failed("encoding.bcd.encode.invalid_input"));
        }
        // The reference's guard is that rounding the value down leaves it
        // unchanged, which is true of an infinity as well as of a whole
        // number -- so an infinity passes here and is encoded, character by
        // character, as the eight letters of its own name.
        if let Some((_, digits, exponent)) = value.parts()
            && !digits.is_empty()
            && exponent < 0
        {
            return Err(failed("encoding.bcd.encode.fractional"));
        }

        let rendered = value.to_fixed();
        let digits = rendered.strip_prefix('-').unwrap_or(&rendered);
        // The sign comes from a numeric comparison against zero, so a zero is
        // written with the *debit* nibble rather than the credit one.
        let positive = !value.is_negative() && !value.is_zero();
        let places = codec::nibbles(digits, &table, packed, signed, positive);

        context.ensure_active()?;
        // Packing decides both halves at once, and they differ: unpacked, the
        // bytes are the nibbles themselves while the nibbles gain a zero half
        // apiece. The reference takes the bytes before it spreads them.
        let (bytes, shown) = if packed {
            (codec::pack(&places), places)
        } else {
            let spread = codec::spread(&places);
            (places, spread)
        };

        let unrenderable = || failed("encoding.bcd.encode.unrenderable");
        let written = match format {
            "Nibbles" => codec::binary(&shown, 4).ok_or_else(unrenderable)?,
            "Bytes" => codec::binary(&bytes, 8).ok_or_else(unrenderable)?,
            // Anything else is raw, which is the reference's own fallthrough
            // rather than a listing of the three names its interface offers.
            _ => codec::raw(&bytes),
        };
        Ok(text_output(written))
    }
}

/// A number read back from binary-coded decimal.
pub struct FromBcd {
    spec: OperationSpec,
}

impl FromBcd {
    /// Creates the From BCD operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.bcd.decode@1",
                display_name: "From BCD",
                category: "Encoding",
                description: "Reads a whole number written with one nibble per decimal digit.",
                cyberchef_alias: Some("From BCD"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Decimal),
                arguments: arguments("How the input is written"),
                inverse: Some("encoding.bcd.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBcd {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBcd {
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
        let (table, packed, signed, format) =
            settings(arguments, "encoding.bcd.decode.unknown_scheme")?;

        let nibbles = if format == "Nibbles" || format == "Bytes" {
            codec::read_binary(&take_text(input)?)
        } else {
            codec::read_raw(&take_bytes(input)?)
        };
        let mut nibbles = if packed {
            nibbles
        } else {
            codec::discard_high(&nibbles)
        };

        context.ensure_active()?;
        let mut output = String::new();
        if signed {
            // The last nibble is consumed whatever it is, and only two values
            // of it mean anything -- so a digit in that place is dropped
            // rather than read.
            if let Some(sign) = nibbles.pop()
                && sign.is_some_and(codec::is_negative_sign)
            {
                output.push('-');
            }
        }

        for nibble in nibbles {
            let Some(nibble) = nibble else {
                return Err(failed("encoding.bcd.decode.invalid_input"));
            };
            let Some(digit) = codec::digit_of(nibble, &table) else {
                return Err(failed("encoding.bcd.decode.not_in_scheme"));
            };
            output.push(char::from(b'0' + digit));
        }

        // Nothing but a sign, or nothing at all, is not a number -- and the
        // reference builds it through the constructor, which throws rather
        // than answering not-a-number.
        DecimalValue::read(&output)
            .map(Value::Decimal)
            .ok_or_else(|| failed("encoding.bcd.decode.invalid_input"))
    }
}
