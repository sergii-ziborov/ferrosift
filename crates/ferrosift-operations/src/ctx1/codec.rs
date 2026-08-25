//! Citrix CTX1 password obfuscation.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::OperationError;

use crate::failure::failed;
use crate::jscompat::string;

/// The constant the scheme xors with at every step.
const MASK: u8 = 0xA5;

/// Where the encoded nibbles start in the alphabet.
const BASE: u8 = 0x41;

/// Obfuscates a password into the CTX1 form.
///
/// Each UTF-16LE byte is xored with `0xA5` and with the previous result, then
/// split into two nibbles written as letters from `A`. The chaining is what
/// makes this not a substitution: the same character encodes differently
/// depending on what came before it.
///
/// This is obfuscation, not encryption. It has no key and is trivially
/// reversible, which is the whole reason the decoder next door exists.
#[must_use]
pub fn encode(input: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 4);
    let mut previous = 0u8;
    for byte in string::to_utf16le(input) {
        previous ^= byte ^ MASK;
        output.push(((previous >> 4) & 0x0f) + BASE);
        output.push((previous & 0x0f) + BASE);
    }
    output
}

/// Recovers a password from the CTX1 form.
///
/// The reference walks the input backwards, because the chain runs forwards
/// and each plaintext byte needs the *encoded* pair that follows it. That is
/// why this reverses twice rather than reading left to right.
///
/// # Errors
///
/// Returns an error when the length is not a multiple of four, or when the
/// recovered bytes are not valid UTF-16.
pub fn decode(input: &[u8]) -> Result<String, OperationError> {
    if !input.len().is_multiple_of(4) {
        return Err(failed("encoding.ctx1.bad_length"));
    }

    let reversed: Vec<u8> = input.iter().rev().copied().collect();
    let mut plain: Vec<u8> = Vec::with_capacity(reversed.len() / 2);
    let mut at = 0;
    while at < reversed.len() {
        // The pair two positions on is the previous link in the forward chain;
        // at the end of the reversed input there is none, so the chain starts
        // from zero.
        let carried = if at + 2 >= reversed.len() {
            0
        } else {
            nibbles(reversed[at + 2], reversed[at + 3])
        };
        plain.push(nibbles(reversed[at], reversed[at + 1]) ^ MASK ^ carried);
        at += 2;
    }

    plain.reverse();
    string::from_utf16le(&plain).ok_or_else(|| failed("encoding.ctx1.not_utf16"))
}

/// Rebuilds one byte from its two encoded letters.
///
/// Masked rather than validated: the reference subtracts and keeps four bits,
/// so a character outside `A`..`P` contributes its low nibble instead of being
/// reported. Reproduced, because a CTX1 hash with a stray character still
/// decodes there and refusing would refuse input the reference accepts.
fn nibbles(low: u8, high: u8) -> u8 {
    (low.wrapping_sub(BASE) & 0x0f) ^ ((high.wrapping_sub(BASE) << 4) & 0xf0)
}
