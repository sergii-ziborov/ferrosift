use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};

use super::codec::{self, Variant};

/// Resolves the checksum variant an argument names.
fn variant(value: &str) -> Result<Variant, OperationError> {
    match value {
        "Bech32" => Ok(Variant::Bech32),
        "Bech32m" => Ok(Variant::Bech32m),
        _ => Err(failed("encoding.bech32.invalid_variant")),
    }
}

/// Reads the operation input as bytes, in whichever form the argument names.
///
/// Written here rather than reached for from the cipher helpers: those live
/// behind the `crypto` feature, and an encoding operation that borrowed them
/// would stop compiling the moment the feature was off -- which is exactly how
/// the ledger build found this.
fn input_bytes(input: Value, hex_input: bool) -> Result<alloc::vec::Vec<u8>, OperationError> {
    let raw = match input {
        Value::Bytes(bytes) => bytes,
        Value::Text(text) => text.text.into_bytes(),
        _ => return Err(OperationError::InvalidArguments),
    };
    if !hex_input {
        return Ok(raw);
    }
    // Whitespace is stripped before the digits are paired, so a hex dump split
    // across lines reads the same as one run of digits.
    let text = core::str::from_utf8(&raw).map_err(|_| failed("encoding.bech32.invalid_hex"))?;
    let digits: alloc::vec::Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.iter().any(|c| !c.is_ascii_hexdigit()) || !digits.len().is_multiple_of(2) {
        return Err(failed("encoding.bech32.invalid_hex"));
    }
    let mut bytes = alloc::vec::Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        let high = pair[0].to_digit(16).unwrap_or(0);
        let low = pair[1].to_digit(16).unwrap_or(0);
        bytes.push(u8::try_from(high * 16 + low).unwrap_or(0));
    }
    Ok(bytes)
}

/// Lower-case hex with no delimiter, which is what every output format uses.
fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        output.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
    }
    output
}

/// Encodes bytes as a Bech32 or Bech32m string.
pub struct ToBech32 {
    spec: OperationSpec,
}

impl ToBech32 {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.bech32.encode@1",
                display_name: "To Bech32",
                category: "Encoding",
                description: "Encodes bytes as Bech32 or Bech32m with a human-readable prefix.",
                cyberchef_alias: Some("To Bech32"),
                input: ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text])),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("hrp", "Human-readable part.", "bc"),
                    text_argument("encoding", "Bech32 or Bech32m.", "Bech32"),
                    text_argument("input_format", "Raw bytes or Hex.", "Raw bytes"),
                    text_argument("mode", "Generic or Bitcoin SegWit.", "Generic"),
                    integer_argument("witness_version", "SegWit witness version, 0 to 16.", 0),
                ],
                inverse: Some("encoding.bech32.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ToBech32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ToBech32 {
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
        let hrp = text_value(arguments, "hrp")?;
        let variant = variant(text_value(arguments, "encoding")?)?;
        let hex_input = text_value(arguments, "input_format")? == "Hex";
        let segwit = text_value(arguments, "mode")? == "Bitcoin SegWit";
        let bytes = input_bytes(input, hex_input)?;

        let text = if segwit {
            // The witness version is prepended to the data and then carried as
            // one 5-bit word, which is why it is an argument rather than part
            // of the input.
            let version = u8::try_from(integer_value(arguments, "witness_version")?)
                .map_err(|_| failed("encoding.bech32.invalid_witness_version"))?;
            let mut with_version = alloc::vec::Vec::with_capacity(bytes.len() + 1);
            with_version.push(version);
            with_version.extend_from_slice(&bytes);
            codec::encode(hrp, &with_version, variant, true, context)?
        } else {
            codec::encode(hrp, &bytes, variant, false, context)?
        };

        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes a Bech32 or Bech32m string.
pub struct FromBech32 {
    spec: OperationSpec,
}

impl FromBech32 {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.bech32.decode@1",
                display_name: "From Bech32",
                category: "Encoding",
                description: "Decodes a Bech32 or Bech32m string to bytes.",
                cyberchef_alias: Some("From Bech32"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("encoding", "Auto-detect, Bech32, or Bech32m.", "Auto-detect"),
                    text_argument(
                        "output_format",
                        "Raw, Hex, Bitcoin scriptPubKey, HRP: Hex, or JSON.",
                        "Hex",
                    ),
                ],
                inverse: Some("encoding.bech32.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for FromBech32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FromBech32 {
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
        let trimmed = input.text.trim();
        // An empty input is an empty output rather than a failure, which the
        // reference decides before it looks at anything else.
        if trimmed.is_empty() {
            return Ok(Value::Text(TextValue {
                text: String::new(),
                encoding: TextEncoding::Utf8,
            }));
        }

        let requested = match text_value(arguments, "encoding")? {
            "Auto-detect" => None,
            other => Some(variant(other)?),
        };
        let decoded = codec::decode(trimmed, requested, context)?;
        let format = text_value(arguments, "output_format")?;

        let text = match format {
            // Each byte becomes the character of the same code point, which is
            // the reference's `String.fromCharCode` over the byte list.
            "Raw" => decoded.data.iter().map(|byte| char::from(*byte)).collect(),
            "Bitcoin scriptPubKey" => script_pub_key(&decoded),
            "HRP: Hex" => {
                let mut text = String::from(decoded.hrp.as_str());
                text.push_str(": ");
                text.push_str(&hex(&decoded.data));
                text
            }
            "JSON" => json(&decoded),
            // Hex is both the named format and the fallback.
            _ => hex(&decoded.data),
        };

        Ok(Value::Text(TextValue {
            text,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Renders a witness program as the script that pays to it.
///
/// Anything that did not decode as `SegWit` falls back to plain hex rather than
/// being reported as an error, so the format is safe to select for input that
/// turns out not to be an address.
fn script_pub_key(decoded: &codec::Decoded) -> String {
    if decoded.witness_version.is_none() || decoded.data.len() < 2 {
        return hex(&decoded.data);
    }
    let version = decoded.data[0];
    let program = &decoded.data[1..];
    let opcode = match version {
        0 => 0x00,
        1..=16 => 0x50 + version,
        _ => return hex(&decoded.data),
    };
    let mut script = alloc::vec::Vec::with_capacity(program.len() + 2);
    script.push(opcode);
    script.push(u8::try_from(program.len()).unwrap_or(0));
    script.extend_from_slice(program);
    hex(&script)
}

/// The JSON form, laid out the way `JSON.stringify(value, null, 2)` does.
fn json(decoded: &codec::Decoded) -> String {
    let mut text = String::from("{\n  \"hrp\": \"");
    text.push_str(&decoded.hrp);
    text.push_str("\",\n  \"encoding\": \"");
    text.push_str(decoded.variant.name());
    text.push_str("\",\n  \"data\": \"");
    text.push_str(&hex(&decoded.data));
    text.push_str("\"\n}");
    text
}
