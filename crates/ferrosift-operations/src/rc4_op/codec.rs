//! Classic RC4 (ARC4) stream cipher.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

pub(super) fn apply(
    input: &[u8],
    key: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    apply_dropping(input, key, 0, context)
}

/// RC4 with the first `drop_words` words of keystream discarded.
///
/// The unit is a 32-bit word, not a byte: the reference generates the
/// keystream four bytes at a time and discards whole words, so a drop of 192
/// skips 768 bytes. Counting in bytes would agree only when the count happens
/// to be a multiple of four.
pub(super) fn apply_dropping(
    input: &[u8],
    key: &[u8],
    drop_words: u64,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let mut s: [u8; 256] = core::array::from_fn(|index| u8::try_from(index).unwrap_or(0));
    let mut j = 0_u8;
    for i in 0..256 {
        // An empty key contributes zero at every position rather than being
        // rejected, which is what the reference does.
        let key_byte = if key.is_empty() {
            0
        } else {
            key[i % key.len()]
        };
        j = j.wrapping_add(s[i]).wrapping_add(key_byte);
        s.swap(i, usize::from(j));
    }
    let mut i = 0_u8;
    j = 0;

    for step in 0..drop_words.saturating_mul(4) {
        if step % 4096 == 0 {
            context.ensure_active()?;
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[usize::from(i)]);
        s.swap(usize::from(i), usize::from(j));
    }

    let mut output = Vec::with_capacity(input.len());
    for (index, &byte) in input.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[usize::from(i)]);
        s.swap(usize::from(i), usize::from(j));
        let k = s[usize::from(s[usize::from(i)].wrapping_add(s[usize::from(j)]))];
        output.push(byte ^ k);
    }
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}
