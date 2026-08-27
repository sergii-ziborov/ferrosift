use alloc::{vec, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jscompat::number::{to_int32, to_uint8};
use crate::key::{INVALID_BYTE_ARRAY, fits_byte_array};

const INVALID_SCHEME: &str = "logic.xor.invalid_scheme";

/// XOR against a repeating key, on the numbers the reference works with.
///
/// The key is not bytes. A Decimal field reaches `bitOp` as whatever `parseInt`
/// returned, and two things here read it as a number rather than as a byte.
///
/// `^` converts both sides with `ToInt32`, so a key of `NaN` behaves as zero
/// and a key of `-1` complements rather than acting as `255` — though the byte
/// that comes out is the same either way, because the result is reduced modulo
/// 256 on the way out and `^` is congruent in the key.
///
/// Null preserving is where that stops being true. It compares `o === k`, an
/// identity test between two JavaScript numbers, so a key of `300` is *not* a
/// byte of `44`: the reference XORs them and writes zero, while a key masked to
/// a byte first would compare equal and pass the byte through untouched.
pub(crate) fn apply(
    input: &[u8],
    key: &[f64],
    scheme: &str,
    null_preserving: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let mut key = if key.is_empty() {
        vec![0.0]
    } else {
        key.to_vec()
    };
    let mut output = Vec::with_capacity(input.len());
    for (index, &operand) in input.iter().enumerate() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let key_index = index % key.len();
        let mut k = key[key_index];
        if scheme == "Cascade" {
            // `input[i + 1] || 0`, which is zero past the end of the input.
            k = f64::from(input.get(index + 1).copied().unwrap_or(0));
        } else if scheme != "Standard"
            && scheme != "Input differential"
            && scheme != "Output differential"
        {
            return Err(failed(INVALID_SCHEME));
        }
        let value = f64::from(operand);
        #[expect(
            clippy::float_cmp,
            reason = "reproducing `o === k`, which is exact equality between two JavaScript numbers"
        )]
        let preserved = null_preserving && (operand == 0 || value == k);
        let result = if preserved {
            value
        } else {
            f64::from(to_int32(value) ^ to_int32(k))
        };
        // The dish refuses the finished array rather than wrapping it, so a key
        // whose ninth bit is set fails every input byte and a negative key fails
        // all of them too — `~o` is negative for every byte.
        if !fits_byte_array(result) {
            return Err(failed(INVALID_BYTE_ARRAY));
        }
        output.push(to_uint8(result));
        if scheme != "Standard" && scheme != "Cascade" && !preserved {
            match scheme {
                // The differential schemes write back into the key array, and
                // what they write is the number rather than the byte — the
                // reference's array holds no narrower type to reduce it to.
                "Input differential" => key[key_index] = value,
                "Output differential" => key[key_index] = result,
                _ => {}
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}
