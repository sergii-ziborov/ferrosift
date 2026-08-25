//! JavaScript string/byte conversions, reproduced exactly.
//!
//! The reference passes data between `string` and `byteArray` operations
//! through two helpers whose behaviour is not the obvious one, and several
//! operations' output depends on which branch they take.

use alloc::{string::String, vec::Vec};

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
pub(crate) fn byte_array_to_utf8(value: &[u8]) -> String {
    core::str::from_utf8(value).map_or_else(
        |_| value.iter().map(|byte| char::from(*byte)).collect(),
        String::from,
    )
}
