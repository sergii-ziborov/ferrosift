use alloc::string::String;
use alloc::vec::Vec;

use crate::jscompat::number::{self as jsint, JsInt};

/// The four ways the reference writes an address.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    DottedDecimal,
    Decimal,
    Octal,
    Hex,
}

/// Resolves a format name, or `None` for one the reference does not know.
pub(crate) fn format(name: &str) -> Option<Format> {
    match name {
        "Dotted Decimal" => Some(Format::DottedDecimal),
        "Decimal" => Some(Format::Decimal),
        "Octal" => Some(Format::Octal),
        "Hex" => Some(Format::Hex),
        _ => None,
    }
}

/// One position of an address, as the reference holds it.
///
/// A *number*, not a byte, and that distinction is the whole of this module's
/// difficulty. The reference reads a dotted address with `parseInt` and pushes
/// whatever comes back into an array without checking it, so a position may
/// hold three hundred, or minus one, or not-a-number. Those are not errors
/// there and they are not errors here; they simply render differently
/// depending on which way the address is being written.
///
/// `None` is not-a-number. Masking it to a byte, which the first draft did,
/// answered `01020300` where the reference answers `010203NaN`.
type Piece = Option<i64>;

/// The positions one line carries, read in the given format.
pub(crate) fn read(line: &str, format: Format) -> Vec<Piece> {
    match format {
        // Split on the point and read each piece on its own. The count is not
        // checked: three pieces stay three, and five stay five.
        Format::DottedDecimal => line
            .split('.')
            .map(|piece| match jsint::parse_wide(piece, 10) {
                JsInt::Value(value) => Some(value),
                JsInt::Nan => None,
            })
            .collect(),
        Format::Decimal => octets(number(line, 10)),
        Format::Octal => octets(number(line, 8)),
        Format::Hex => hex_bytes(line),
    }
}

/// A whole number read the way `parseInt` reads one.
fn number(line: &str, radix: u32) -> Option<i64> {
    match jsint::parse_wide(line, radix) {
        JsInt::Value(value) => Some(value),
        JsInt::Nan => None,
    }
}

/// The four positions of a single value, from thirty-two bit shifts.
///
/// The reference masks each with `& 255` here, so these four are always bytes
/// -- unlike the dotted reading, which masks nothing.
fn octets(value: Option<i64>) -> Vec<Piece> {
    let word = to_int32(value);
    alloc::vec![
        Some(i64::from(word >> 24 & 255)),
        Some(i64::from(word >> 16 & 255)),
        Some(i64::from(word >> 8 & 255)),
        Some(i64::from(word & 255)),
    ]
}

/// Bytes from a run of hexadecimal digits, two at a time.
///
/// Mirrors the reference's `fromHex` with its automatic delimiter, and two
/// details of it are easy to get wrong.
///
/// A lone digit at the end is a byte, not a discard. The reference steps by
/// two and reads whatever is left, so seven digits are four bytes and the last
/// is worth its single digit -- `c0a8000` is `192.168.0.0`, not `192.168.0`.
///
/// A `0x` is removed whole. The reference's pattern is "anything that is not a
/// hexadecimal digit, *or* `0x`", and the alternation reaches the second arm
/// only because the zero survives the first -- so keeping the zero and
/// dropping the `x`, which a plain digit filter does, leaves an extra digit
/// and shifts every byte after it.
fn hex_bytes(line: &str) -> Vec<Piece> {
    let mut digits = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if !character.is_ascii_hexdigit() {
            continue;
        }
        if character == '0' && matches!(characters.peek(), Some('x' | 'X')) {
            characters.next();
            continue;
        }
        digits.push(character);
    }

    digits
        .as_bytes()
        .chunks(2)
        .filter_map(|pair| {
            let text = core::str::from_utf8(pair).ok()?;
            u8::from_str_radix(text, 16)
                .ok()
                .map(|byte| Some(i64::from(byte)))
        })
        .collect()
}

/// What JavaScript's shift operators see: the value truncated to thirty-two
/// bits, with not-a-number becoming zero.
fn to_int32(value: Option<i64>) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the truncation is the reference's own, from a JavaScript shift"
    )]
    match value {
        Some(value) => value as i32,
        None => 0,
    }
}

/// Writes the positions in the given format.
pub(crate) fn write(pieces: &[Piece], format: Format) -> String {
    match format {
        // Every position it was given, however many and whatever they hold.
        Format::DottedDecimal => {
            let mut output = String::new();
            for piece in pieces {
                if !output.is_empty() {
                    output.push('.');
                }
                output.push_str(&decimal_text(*piece));
            }
            output
        }
        Format::Decimal => alloc::format!("{}", word(pieces)),
        // The leading zero is the reference's, and is what makes the answer
        // readable back by the octal reading.
        Format::Octal => alloc::format!("0{:o}", word(pieces)),
        // Two digits per position, but only where two are enough. The
        // reference pads to two and does not truncate, so a position above a
        // byte is three digits and not-a-number is the three letters of its
        // own name.
        Format::Hex => {
            let mut output = String::new();
            for piece in pieces {
                output.push_str(&hex_text(*piece));
            }
            output
        }
    }
}

/// One position as JavaScript renders a number in a string concatenation.
fn decimal_text(piece: Piece) -> String {
    piece.map_or_else(|| String::from("NaN"), |value| alloc::format!("{value}"))
}

/// One position as `Utils.hex` renders it: base sixteen, padded to two.
fn hex_text(piece: Piece) -> String {
    let Some(value) = piece else {
        return String::from("NaN");
    };
    let text = if value < 0 {
        alloc::format!("-{:x}", value.unsigned_abs())
    } else {
        alloc::format!("{value:x}")
    };
    if text.len() >= 2 {
        return text;
    }
    let mut padded = String::from("0");
    padded.push_str(&text);
    padded
}

/// The first four positions packed into one word.
///
/// A position the address does not have is `undefined` in the reference, which
/// its shift reads as zero -- so a short address packs with zeros rather than
/// failing.
fn word(pieces: &[Piece]) -> u32 {
    let at = |index: usize| to_int32(pieces.get(index).copied().flatten());
    let packed = at(0).wrapping_shl(24) | at(1).wrapping_shl(16) | at(2).wrapping_shl(8) | at(3);
    #[expect(
        clippy::cast_sign_loss,
        reason = "the reference's `>>> 0` reads the same bits as unsigned"
    )]
    let unsigned = packed as u32;
    unsigned
}
