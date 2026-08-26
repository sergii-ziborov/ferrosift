//! XXTEA, the corrected Block TEA of Wheeler and Needham.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The round constant, a scaled golden ratio.
const DELTA: u32 = 0x9e37_79b9;

/// The shortest key the cipher accepts; a shorter one is zero-extended.
const KEY_BYTES: usize = 16;

/// Packs bytes into words, little-endian, optionally appending the length.
///
/// The length word is what lets decryption recover the exact input size, so a
/// message whose length is not a multiple of four survives a round trip.
fn to_words(bytes: &[u8], include_length: bool) -> Vec<u32> {
    let count = bytes.len().div_ceil(4);
    let mut words = alloc::vec![0_u32; if include_length { count + 1 } else { count }];
    if include_length {
        words[count] = u32::try_from(bytes.len()).unwrap_or(0);
    }
    for (index, byte) in bytes.iter().enumerate() {
        words[index >> 2] |= u32::from(*byte) << ((index & 3) << 3);
    }
    words
}

/// Unpacks words back into bytes, optionally trusting the trailing length.
///
/// A length that does not sit within three bytes of the padded size means the
/// ciphertext was not produced by this cipher with this key -- the last word
/// is then not a length at all, and there is nothing to return.
fn to_bytes(words: &[u32], include_length: bool) -> Option<Vec<u8>> {
    let mut count = words.len() << 2;
    if include_length {
        let claimed = u64::from(*words.last()?);
        count -= 4;
        let padded = count as u64;
        if claimed + 3 < padded || claimed > padded {
            return None;
        }
        count = usize::try_from(claimed).ok()?;
    }
    let mut bytes = Vec::with_capacity(count);
    for index in 0..count {
        bytes.push(((words[index >> 2] >> ((index & 3) << 3)) & 0xff) as u8);
    }
    Some(bytes)
}

/// The mixing function, in the exact shape and precedence the reference uses.
///
/// Every operation here is on a 32-bit pattern. The two additions wrap rather
/// than widen, which is what the reference's outermost `^` does to the
/// floating-point sums either side of it.
fn mix(sum: u32, y: u32, z: u32, p: usize, e: u32, key: &[u32]) -> u32 {
    // Both operands are already below four, so the index selects one of the
    // key's first four words without a mask of its own. The narrowing is
    // exact for the same reason: `p & 3` cannot exceed three.
    let index = (u32::try_from(p & 3).unwrap_or(0) ^ e) as usize;
    let left = (z >> 5) ^ (y << 2);
    let right = (y >> 3) ^ (z << 4);
    let keyed = (sum ^ y).wrapping_add(key[index] ^ z);
    left.wrapping_add(right) ^ keyed
}

/// How many rounds a message of this many words takes.
fn rounds(length: usize) -> u32 {
    // Integer division on purpose: the reference floors the quotient before
    // adding it to six.
    6 + 52 / u32::try_from(length).unwrap_or(u32::MAX)
}

/// Encrypts in place.
fn encrypt_words(
    words: &mut [u32],
    key: &[u32],
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let length = words.len();
    let last = length - 1;
    let mut z = words[last];
    let mut sum: u32 = 0;
    for _ in 0..rounds(length) {
        context.ensure_active()?;
        sum = sum.wrapping_add(DELTA);
        let e = (sum >> 2) & 3;
        for p in 0..last {
            let y = words[p + 1];
            words[p] = words[p].wrapping_add(mix(sum, y, z, p, e, key));
            z = words[p];
        }
        let y = words[0];
        words[last] = words[last].wrapping_add(mix(sum, y, z, last, e, key));
        z = words[last];
    }
    Ok(())
}

/// Decrypts in place, unwinding the rounds in the order they were applied.
fn decrypt_words(
    words: &mut [u32],
    key: &[u32],
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    let length = words.len();
    let last = length - 1;
    let mut y = words[0];
    let mut sum = rounds(length).wrapping_mul(DELTA);
    while sum != 0 {
        context.ensure_active()?;
        let e = (sum >> 2) & 3;
        for p in (1..=last).rev() {
            let z = words[p - 1];
            words[p] = words[p].wrapping_sub(mix(sum, y, z, p, e, key));
            y = words[p];
        }
        let z = words[last];
        words[0] = words[0].wrapping_sub(mix(sum, y, z, 0, e, key));
        y = words[0];
        sum = sum.wrapping_sub(DELTA);
    }
    Ok(())
}

/// Zero-extends a short key to the sixteen bytes the cipher needs.
fn fix_key(key: &[u8]) -> Vec<u32> {
    if key.len() < KEY_BYTES {
        let mut padded = alloc::vec![0_u8; KEY_BYTES];
        padded[..key.len()].copy_from_slice(key);
        return to_words(&padded, false);
    }
    to_words(key, false)
}

/// Encrypts `data`, returning it unchanged when it is empty.
pub(super) fn encrypt(
    data: &[u8],
    key: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let mut words = to_words(data, true);
    encrypt_words(&mut words, &fix_key(key), context)?;
    to_bytes(&words, false).ok_or_else(|| failed("crypto.xxtea.malformed"))
}

/// Decrypts `data`, returning it unchanged when it is empty.
pub(super) fn decrypt(
    data: &[u8],
    key: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let mut words = to_words(data, false);
    decrypt_words(&mut words, &fix_key(key), context)?;
    to_bytes(&words, true).ok_or_else(|| failed("crypto.xxtea.malformed"))
}
