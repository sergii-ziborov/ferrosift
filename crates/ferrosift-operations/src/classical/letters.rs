//! Alphabet handling shared by the classical ciphers.
//!
//! Every one of them indexes a JavaScript string, which means UTF-16 code
//! units rather than characters. That distinction is only visible for astral
//! input, but where it is visible it changes the answer — Vigenère counts
//! skipped units to keep its key in step, so an emoji advances the key by two.

use alloc::{string::String, vec::Vec};

/// Splits a string into UTF-16 code units, the unit these ciphers index by.
pub(super) fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

/// Rebuilds a string from code units, replacing any unpaired surrogate.
///
/// Surrogates are never transformed by these ciphers, so a pair that went in
/// adjacent comes out adjacent and is decoded back to its original character.
pub(super) fn from_units(value: &[u16]) -> String {
    char::decode_utf16(value.iter().copied())
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Position of a lower-case ASCII letter in the alphabet.
pub(super) const fn lower_index(unit: u16) -> Option<usize> {
    if unit >= b'a' as u16 && unit <= b'z' as u16 {
        Some((unit - b'a' as u16) as usize)
    } else {
        None
    }
}

/// Position of the code unit's lower-case form, when it has one in `a-z`.
///
/// The reference reaches this through `String.prototype.toLowerCase`, whose
/// full Unicode mapping lands in `a-z` for exactly two code points outside
/// ASCII: U+0130 LATIN CAPITAL LETTER I WITH DOT ABOVE, which lowercases to
/// `i` plus a combining dot, and U+212A KELVIN SIGN, which lowercases to `k`.
/// Everything else that matters is plain ASCII.
pub(super) const fn lowered_index(unit: u16) -> Option<usize> {
    match unit {
        0x0041..=0x005A => Some((unit - 0x0041) as usize),
        0x212A => Some(10),
        _ => lower_index(unit),
    }
}

/// The upper-case form of an alphabet position.
pub(super) fn upper_unit(index: usize) -> u16 {
    u16::from(b'A') + u16::try_from(index % 26).unwrap_or(0)
}

/// The lower-case form of an alphabet position.
pub(super) fn lower_unit(index: usize) -> u16 {
    u16::from(b'a') + u16::try_from(index % 26).unwrap_or(0)
}

/// Greatest common divisor, for the affine coprimality check.
pub(super) const fn gcd(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.abs()
}

/// Modular inverse of `value` under `modulus`, by trial.
///
/// The modulus here is always 26, so a search is both correct and immediate.
pub(super) fn mod_inverse(value: i128, modulus: i128) -> Option<i128> {
    (1..modulus).find(|candidate| (value * candidate).rem_euclid(modulus) == 1)
}
