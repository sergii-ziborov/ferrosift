use alloc::collections::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    ArgumentSpec, Arguments, OperationClassification, OperationSpec, Value, ValueConstraint,
    ValueKind,
};

use crate::args::{
    integer_argument, integer_value, map_argument, map_value, text_argument, text_value,
    toggle_string_default, toggle_string_parts,
};
use crate::failure::failed;
use crate::hex_util::to_hex_lower;
use crate::jscompat::string::byte_array_to_utf8;
use crate::key::{convert_to_byte_array, stored_as_bytes};
use crate::spec::{SpecDefinition, build};
use crate::value::text;

use super::codec::{self, BLOCK, Variant};

const INVALID_KEY: &str = "crypto.tea.invalid_key";
const INVALID_IV: &str = "crypto.tea.invalid_iv";
const INVALID_ROUNDS: &str = "crypto.tea.invalid_rounds";
const INVALID_FORMAT: &str = "crypto.tea.invalid_format";

/// A hundred and twenty-eight bits, in bytes. Neither cipher takes any other.
const KEY_LENGTH: usize = 16;

/// Which direction an instance runs.
#[derive(Clone, Copy)]
enum Direction {
    Encrypt,
    Decrypt,
}

/// Which cipher, before its cycle count is read from the arguments.
#[derive(Clone, Copy)]
enum Family {
    Tea,
    Xtea,
}

/// TEA and XTEA, in both directions.
///
/// One struct for four operations, because the four differ only in which block
/// function they name and whether they expose a cycle count. The reference has
/// four files sharing one library; this has one type sharing one codec.
pub struct Tea {
    spec: OperationSpec,
    family: Family,
    direction: Direction,
}

impl Tea {
    /// TEA, encrypting.
    #[must_use]
    pub fn encrypt() -> Self {
        Self::new(Family::Tea, Direction::Encrypt)
    }

    /// TEA, decrypting.
    #[must_use]
    pub fn decrypt() -> Self {
        Self::new(Family::Tea, Direction::Decrypt)
    }

    /// XTEA, encrypting.
    #[must_use]
    pub fn xtea_encrypt() -> Self {
        Self::new(Family::Xtea, Direction::Encrypt)
    }

    /// XTEA, decrypting.
    #[must_use]
    pub fn xtea_decrypt() -> Self {
        Self::new(Family::Xtea, Direction::Decrypt)
    }

    fn new(family: Family, direction: Direction) -> Self {
        let (id, name, inverse) = match (family, direction) {
            (Family::Tea, Direction::Encrypt) => (
                "crypto.tea.encrypt@1",
                "TEA Encrypt",
                "crypto.tea.decrypt@1",
            ),
            (Family::Tea, Direction::Decrypt) => (
                "crypto.tea.decrypt@1",
                "TEA Decrypt",
                "crypto.tea.encrypt@1",
            ),
            (Family::Xtea, Direction::Encrypt) => (
                "crypto.xtea.encrypt@1",
                "XTEA Encrypt",
                "crypto.xtea.decrypt@1",
            ),
            (Family::Xtea, Direction::Decrypt) => (
                "crypto.xtea.decrypt@1",
                "XTEA Decrypt",
                "crypto.xtea.encrypt@1",
            ),
        };
        let description = match (family, direction) {
            (Family::Tea, Direction::Encrypt) => "Encrypts bytes with the TEA block cipher.",
            (Family::Tea, Direction::Decrypt) => "Decrypts bytes encrypted with TEA.",
            (Family::Xtea, Direction::Encrypt) => "Encrypts bytes with the XTEA block cipher.",
            (Family::Xtea, Direction::Decrypt) => "Decrypts bytes encrypted with XTEA.",
        };

        let mut arguments = vec![
            map_argument(
                "key",
                "Key as a CyberChef toggleString; exactly 16 bytes.",
                toggle_string_default("Hex", ""),
            ),
            map_argument(
                "iv",
                "Initialisation vector; empty defaults to 8 null bytes.",
                toggle_string_default("Hex", ""),
            ),
            text_argument("mode", "CBC, CFB, OFB, CTR, or ECB.", "CBC"),
            text_argument("input", "Input encoding: Raw or Hex.", "Raw"),
            text_argument("output", "Output encoding: Hex or Raw.", "Hex"),
            text_argument("padding", "PKCS5, NO, ZERO, RANDOM, or BIT.", "PKCS5"),
        ];
        if matches!(family, Family::Xtea) {
            arguments.push(cycles_argument());
        }

        Self {
            spec: build(SpecDefinition {
                id,
                display_name: name,
                category: "Ciphers",
                description,
                cyberchef_alias: Some(name),
                input: text_kinds(),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments,
                inverse: Some(inverse),
                classifications: Some(&[OperationClassification::Unsafe]),
            }),
            family,
            direction,
        }
    }
}

/// XTEA's cycle count, which TEA does not have.
fn cycles_argument() -> ArgumentSpec {
    integer_argument("rounds", "Cycles, 1 to 255; the standard is 32.", 32)
}

fn text_kinds() -> ValueConstraint {
    ValueConstraint::OneOf(BTreeSet::from([ValueKind::Bytes, ValueKind::Text]))
}

impl Operation for Tea {
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
        let (key_option, key_string) = toggle_string_parts(map_value(arguments, "key")?)?;
        let (iv_option, iv_string) = toggle_string_parts(map_value(arguments, "iv")?)?;
        let key = stored_as_bytes(&convert_to_byte_array(key_string, key_option));
        let iv = stored_as_bytes(&convert_to_byte_array(iv_string, iv_option));
        let mode = text_value(arguments, "mode")?;
        let padding = text_value(arguments, "padding")?;
        let input_format = text_value(arguments, "input")?;
        let output_format = text_value(arguments, "output")?;

        if key.len() != KEY_LENGTH {
            return Err(failed(INVALID_KEY));
        }
        // Outside ECB the IV is either the block length or absent. Inside ECB
        // the reference does not look at it, so nor does this — an IV left over
        // from another mode is ignored rather than refused.
        if mode != "ECB" && iv.len() != BLOCK && !iv.is_empty() {
            return Err(failed(INVALID_IV));
        }

        let variant = match self.family {
            Family::Tea => Variant::Tea,
            Family::Xtea => Variant::Xtea(cycles(arguments)?),
        };
        let message = read_input(input, input_format)?;
        let output = match self.direction {
            Direction::Encrypt => {
                codec::encrypt(&message, variant, &key, &iv, mode, padding, context)?
            }
            Direction::Decrypt => {
                codec::decrypt(&message, variant, &key, &iv, mode, padding, context)?
            }
        };

        context.ensure_active()?;
        write_output(&output, output_format)
    }
}

/// XTEA's cycle count, checked the way its interface declares it.
fn cycles(arguments: &Arguments) -> Result<u32, OperationError> {
    let rounds = integer_value(arguments, "rounds")?;
    if !(1..=255).contains(&rounds) {
        return Err(failed(INVALID_ROUNDS));
    }
    u32::try_from(rounds).map_err(|_| failed(INVALID_ROUNDS))
}

/// Reads the input the way the reference reads it: as a string, converted.
///
/// The *array* reading, which is where this differs from AES beside it — the
/// two operations call different `Utils` functions and `key.rs` says what that
/// changes. Bytes that arrive as bytes stay as they are; nothing is re-encoded
/// on the way in.
fn read_input(input: Value, format: &str) -> Result<Vec<u8>, OperationError> {
    let text = match input {
        Value::Bytes(bytes) if format == "Raw" => return Ok(bytes),
        Value::Bytes(bytes) => byte_array_to_utf8(&bytes),
        Value::Text(value) => value.text,
        _ => return Err(OperationError::InvalidArguments),
    };
    Ok(stored_as_bytes(&convert_to_byte_array(&text, format)))
}

/// Writes the output as hex, or as the *string* the reference makes of it.
///
/// `Raw` is not the bytes. The reference hands them to `byteArrayToUtf8`, which
/// decodes them as UTF-8 and falls back to one character per byte when they are
/// not valid UTF-8 — so the result is text either way, and a recipe reading it
/// back gets that text rather than the ciphertext.
fn write_output(output: &[u8], format: &str) -> Result<Value, OperationError> {
    match format {
        "Hex" => Ok(text(to_hex_lower(output))),
        "Raw" => Ok(text(byte_array_to_utf8(output))),
        _ => Err(failed(INVALID_FORMAT)),
    }
}
