//! ROT47 over printable ASCII, and ROT8000 over the printable BMP.

use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use super::letters::{from_units, units};

/// Rotates the 94 printable ASCII characters from `!` to `~`.
///
/// A zero amount is a no-op — the reference guards the whole loop with
/// `if (amount)` — and a negative amount is folded into the positive range
/// before rotating.
pub(super) fn rot47(
    input: &[u8],
    amount: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let mut output = input.to_vec();
    if amount == 0 {
        return Ok(output);
    }
    let amount = if amount < 0 {
        94 - (amount.unsigned_abs() % 94)
    } else {
        u128::try_from(amount).unwrap_or(0)
    };
    for (position, byte) in output.iter_mut().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if (33..=126).contains(byte) {
            let rotated = (u128::from(*byte) - 33 + amount) % 94;
            *byte = u8::try_from(rotated + 33).unwrap_or(*byte);
        }
    }
    context.ensure_active()?;
    Ok(output)
}

/// The code-point transitions that bound ROT8000's rotatable set.
///
/// Each entry switches validity on or off from that code point upward. The
/// table is the reference's own `valid-code-point-transitions` data, kept
/// verbatim so the rotation lands on the same partner character; deriving the
/// ranges from Unicode categories instead would drift from it.
const TRANSITIONS: [(u32, bool); 17] = [
    (33, true),
    (127, false),
    (161, true),
    (5760, false),
    (5761, true),
    (8192, false),
    (8203, true),
    (8232, false),
    (8234, true),
    (8239, false),
    (8240, true),
    (8287, false),
    (8288, true),
    (12288, false),
    (12289, true),
    // The surrogate range is excluded, and everything above it is valid again.
    (55296, false),
    (57344, true),
];

/// Builds the ordered list of rotatable code units.
fn rotatable() -> Vec<u16> {
    let mut valid = Vec::new();
    let mut current = false;
    for code in 0u32..0x1_0000 {
        if let Some((_, state)) = TRANSITIONS.iter().find(|(start, _)| *start == code) {
            current = *state;
        }
        if current && let Ok(unit) = u16::try_from(code) {
            valid.push(unit);
        }
    }
    valid
}

/// Rotates each code unit halfway through the rotatable set.
///
/// This works on UTF-16 code units, not characters, and the surrogate range is
/// excluded from the set — so an astral character passes through as its two
/// unchanged halves and survives the round trip.
pub(super) fn rot8000(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let valid = rotatable();
    let half = valid.len() / 2;
    let mut output: Vec<u16> = Vec::with_capacity(input.len());
    for (position, unit) in units(input).iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        match valid.binary_search(unit) {
            Ok(index) => output.push(valid[(index + half) % valid.len()]),
            Err(_) => output.push(*unit),
        }
    }
    context.ensure_active()?;
    Ok(from_units(&output))
}
