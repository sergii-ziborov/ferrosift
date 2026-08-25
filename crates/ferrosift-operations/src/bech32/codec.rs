//! Bech32 (BIP-173) and Bech32m (BIP-350).

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The 32 data symbols, in value order.
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Checksum constant that distinguishes Bech32 from Bech32m.
const BECH32_CONST: u32 = 1;
const BECH32M_CONST: u32 = 0x2bc8_30a3;

/// Generator coefficients of the BCH code.
const GENERATOR: [u32; 5] = [
    0x3b6a_57b2,
    0x2650_8e6d,
    0x1ea1_19fa,
    0x3d42_33dd,
    0x2a14_62b3,
];

/// An encoded string may never exceed this, checked after it is built.
const MAX_LENGTH: usize = 90;

/// The two checksum variants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Variant {
    Bech32,
    Bech32m,
}

impl Variant {
    fn constant(self) -> u32 {
        match self {
            Self::Bech32 => BECH32_CONST,
            Self::Bech32m => BECH32M_CONST,
        }
    }

    /// The spelling the JSON output uses.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Bech32 => "Bech32",
            Self::Bech32m => "Bech32m",
        }
    }
}

/// What a decode recovered.
pub(super) struct Decoded {
    pub(super) hrp: String,
    pub(super) data: Vec<u8>,
    pub(super) variant: Variant,
    /// `Some` only when the address was read as `SegWit`.
    pub(super) witness_version: Option<u8>,
}

/// The BCH checksum over a sequence of 5-bit values.
fn polymod(values: &[u8]) -> u32 {
    let mut checksum: u32 = 1;
    for value in values {
        let top = checksum >> 25;
        checksum = ((checksum & 0x01ff_ffff) << 5) ^ u32::from(*value);
        for (index, coefficient) in GENERATOR.iter().enumerate() {
            if (top >> index) & 1 == 1 {
                checksum ^= coefficient;
            }
        }
    }
    checksum
}

/// Expands the human-readable part into the values the checksum covers.
///
/// High bits first, a zero separator, then low bits -- so the checksum depends
/// on the whole prefix rather than on its low five bits alone.
fn hrp_expand(hrp: &str) -> Vec<u8> {
    let bytes = hrp.as_bytes();
    let mut expanded = Vec::with_capacity(bytes.len() * 2 + 1);
    expanded.extend(bytes.iter().map(|byte| byte >> 5));
    expanded.push(0);
    expanded.extend(bytes.iter().map(|byte| byte & 31));
    expanded
}

/// Regroups bytes into 5-bit words, padding the last group with zeroes.
fn to_words(data: &[u8]) -> Vec<u8> {
    let mut accumulator: u32 = 0;
    let mut bits = 0_u32;
    let mut words = Vec::with_capacity(data.len() * 8 / 5 + 2);
    for byte in data {
        accumulator = (accumulator << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            words.push(((accumulator >> bits) & 31) as u8);
        }
    }
    if bits > 0 {
        words.push(((accumulator << (5 - bits)) & 31) as u8);
    }
    words
}

/// Regroups 5-bit words back into bytes, rejecting malformed padding.
///
/// Five or more leftover bits would be a byte that was never finished, and any
/// leftover bit that is set was not padding. Both are refused rather than
/// discarded, which is what stops two different strings decoding alike.
fn from_words(words: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut accumulator: u32 = 0;
    let mut bits = 0_u32;
    let mut bytes = Vec::with_capacity(words.len() * 5 / 8 + 1);
    for word in words {
        accumulator = (accumulator << 5) | u32::from(*word);
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            bytes.push(((accumulator >> bits) & 255) as u8);
        }
    }
    if bits >= 5 {
        return Err(failed("encoding.bech32.padding_too_long"));
    }
    if bits > 0 && (accumulator << (8 - bits)) & 255 != 0 {
        return Err(failed("encoding.bech32.padding_not_zero"));
    }
    Ok(bytes)
}

/// Builds the six checksum words for a payload.
fn create_checksum(hrp: &str, words: &[u8], variant: Variant) -> [u8; 6] {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(words);
    values.extend_from_slice(&[0; 6]);
    let modulus = polymod(&values) ^ variant.constant();
    core::array::from_fn(|index| {
        let shift = 5 * (5 - u32::try_from(index).unwrap_or(0));
        ((modulus >> shift) & 31) as u8
    })
}

/// Whether a payload's checksum matches the given variant.
fn verify_checksum(hrp: &str, data: &[u8], variant: Variant) -> bool {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    polymod(&values) == variant.constant()
}

/// Encodes `data` under `hrp`.
///
/// In `SegWit` mode the first byte is the witness version and is carried as a
/// single 5-bit word rather than being regrouped with the rest -- that is what
/// makes an address round-trip. The mode only takes effect from two bytes up;
/// below that the reference falls through to the generic path rather than
/// reporting the input as too short, and that fall-through is reproduced.
pub(super) fn encode(
    hrp: &str,
    data: &[u8],
    variant: Variant,
    segwit: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if hrp.is_empty() {
        return Err(failed("encoding.bech32.empty_hrp"));
    }
    if hrp.bytes().any(|byte| !(33..=126).contains(&byte)) {
        return Err(failed("encoding.bech32.invalid_hrp"));
    }
    let lowered = hrp.to_ascii_lowercase();

    let words = if segwit && data.len() >= 2 {
        let version = data[0];
        if version > 16 {
            return Err(failed("encoding.bech32.invalid_witness_version"));
        }
        let program = &data[1..];
        if program.len() < 2 || program.len() > 40 {
            return Err(failed("encoding.bech32.invalid_program_length"));
        }
        if version == 0 && program.len() != 20 && program.len() != 32 {
            return Err(failed("encoding.bech32.invalid_program_length"));
        }
        let mut words = Vec::with_capacity(program.len() * 8 / 5 + 3);
        words.push(version);
        words.extend(to_words(program));
        words
    } else {
        to_words(data)
    };

    context.ensure_active()?;
    let checksum = create_checksum(&lowered, &words, variant);
    let mut output = String::with_capacity(lowered.len() + 1 + words.len() + 6);
    output.push_str(&lowered);
    output.push('1');
    for word in words.iter().chain(checksum.iter()) {
        output.push(char::from(CHARSET[usize::from(*word) & 31]));
    }

    // Checked on the finished string, not predicted from the input, because
    // that is where the reference checks it.
    if output.len() > MAX_LENGTH {
        return Err(failed("encoding.bech32.too_long"));
    }
    Ok(output)
}

/// The prefixes the decoder will try to read as `SegWit`.
const SEGWIT_HRPS: [&str; 5] = ["bc", "tb", "ltc", "tltc", "bcrt"];

/// Decodes a Bech32 or Bech32m string.
pub(super) fn decode(
    input: &str,
    requested: Option<Variant>,
    context: &OperationContext<'_>,
) -> Result<Decoded, OperationError> {
    context.ensure_active()?;
    if input.is_empty() {
        return Err(failed("encoding.bech32.empty_input"));
    }
    if input.chars().count() > MAX_LENGTH {
        return Err(failed("encoding.bech32.too_long"));
    }
    // Case carries no information but mixing it does: a string that is partly
    // upper and partly lower has been edited, and the checksum would hide it.
    let upper = input.bytes().any(|byte| byte.is_ascii_uppercase());
    let lower = input.bytes().any(|byte| byte.is_ascii_lowercase());
    if upper && lower {
        return Err(failed("encoding.bech32.mixed_case"));
    }
    let lowered = input.to_ascii_lowercase();

    // The separator is the *last* `1`, so the prefix may contain one.
    let separator = lowered
        .rfind('1')
        .ok_or_else(|| failed("encoding.bech32.no_separator"))?;
    if separator == 0 {
        return Err(failed("encoding.bech32.empty_hrp"));
    }
    if separator + 7 > lowered.len() {
        return Err(failed("encoding.bech32.data_too_short"));
    }

    let hrp = &lowered[..separator];
    if hrp.bytes().any(|byte| !(33..=126).contains(&byte)) {
        return Err(failed("encoding.bech32.invalid_hrp"));
    }

    let mut data = Vec::with_capacity(lowered.len() - separator - 1);
    for byte in lowered[separator + 1..].bytes() {
        let value = CHARSET
            .iter()
            .position(|symbol| *symbol == byte)
            .ok_or_else(|| failed("encoding.bech32.invalid_character"))?;
        data.push(u8::try_from(value).unwrap_or(0));
    }

    context.ensure_active()?;
    let variant = match requested {
        Some(variant) => {
            if !verify_checksum(hrp, &data, variant) {
                return Err(failed("encoding.bech32.bad_checksum"));
            }
            variant
        }
        // Bech32 is tried first, so a string valid under both is reported as
        // the older one.
        None => {
            if verify_checksum(hrp, &data, Variant::Bech32) {
                Variant::Bech32
            } else if verify_checksum(hrp, &data, Variant::Bech32m) {
                Variant::Bech32m
            } else {
                return Err(failed("encoding.bech32.bad_checksum"));
            }
        }
    };

    let words = &data[..data.len() - 6];

    // A SegWit read is attempted only for the prefixes that use it, and only
    // when the result is a well-formed witness program. Anything else falls
    // back to a plain regroup rather than failing, so a `bc1` string that is
    // not an address still decodes.
    let segwit = SEGWIT_HRPS.contains(&hrp) && words.first().is_some_and(|word| *word <= 16);
    if segwit {
        let version = words[0];
        if let Ok(program) = from_words(&words[1..]) {
            let valid = if version == 0 {
                program.len() == 20 || program.len() == 32
            } else {
                (2..=40).contains(&program.len())
            };
            if valid {
                let mut bytes = Vec::with_capacity(program.len() + 1);
                bytes.push(version);
                bytes.extend_from_slice(&program);
                return Ok(Decoded {
                    hrp: String::from(hrp),
                    data: bytes,
                    variant,
                    witness_version: Some(version),
                });
            }
        }
    }

    Ok(Decoded {
        hrp: String::from(hrp),
        data: from_words(words)?,
        variant,
        witness_version: None,
    })
}
