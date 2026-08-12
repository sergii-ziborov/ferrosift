//! Classic RC4 (ARC4) stream cipher.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

pub(super) fn apply(
    input: &[u8],
    key: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let mut s: [u8; 256] = core::array::from_fn(|index| u8::try_from(index).unwrap_or(0));
    let mut j = 0_u8;
    if key.is_empty() {
        // CryptoJS still initializes with empty key as all zeros contribution.
    }
    for i in 0..256 {
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
