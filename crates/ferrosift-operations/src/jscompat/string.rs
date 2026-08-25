//! JavaScript string/byte conversions, reproduced exactly.
//!
//! The reference passes data between `string` and `byteArray` operations
//! through two helpers whose behaviour is not the obvious one, and several
//! operations' output depends on which branch they take.

use alloc::{string::String, vec::Vec};

/// UTF-16 little-endian bytes, as Windows and Citrix both mean by "Unicode".
///
/// Two bytes per code unit, so an astral character contributes four. That
/// matters for NT Hash, where the digest is over these exact bytes: hashing
/// UTF-8 instead would agree on ASCII and differ on everything else.
pub(crate) fn to_utf16le(value: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len() * 2);
    for unit in value.encode_utf16() {
        output.extend_from_slice(&unit.to_le_bytes());
    }
    output
}

/// The inverse, or `None` when the units are not well-formed UTF-16.
///
/// JavaScript would hand back a string containing a lone surrogate here; Rust
/// has no such string, so this reports that it cannot rather than substituting
/// a replacement character and claiming a successful decode. A caller that
/// hits this is looking at input the reference would have mangled.
pub(crate) fn from_utf16le(bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

/// `Utils.strToByteArray`.
///
/// Takes UTF-16 code units directly when every one fits in a byte, and only
/// falls back to UTF-8 encoding when one does not — so `"é"` (U+00E9) becomes
/// the single byte `0xE9`, not the two bytes UTF-8 would give.
pub(crate) fn str_to_byte_array(value: &str) -> Vec<u8> {
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.iter().all(|unit| *unit <= 255) {
        return units
            .into_iter()
            .map(|unit| u8::try_from(unit).unwrap_or(0))
            .collect();
    }
    value.as_bytes().to_vec()
}

/// `Utils.byteArrayToUtf8`.
///
/// A strict UTF-8 decode, falling back to Latin-1 for the whole buffer when
/// any part of it is not valid UTF-8. The fallback is all-or-nothing, so a
/// single bad byte changes how every other byte is read.
///
/// A leading byte-order mark is dropped on the successful path and kept on the
/// fallback. That is not a choice made here: the reference decodes with a
/// `TextDecoder`, whose default `ignoreBOM: false` means *remove* the mark, and
/// its Latin-1 fallback has no such notion. So `ef bb bf` decodes to the empty
/// string while `ef bb` — which is not valid UTF-8 — decodes to two characters.
pub(crate) fn byte_array_to_utf8(value: &[u8]) -> String {
    core::str::from_utf8(value).map_or_else(
        |_| value.iter().map(|byte| char::from(*byte)).collect(),
        |text| String::from(text.strip_prefix('\u{feff}').unwrap_or(text)),
    )
}
