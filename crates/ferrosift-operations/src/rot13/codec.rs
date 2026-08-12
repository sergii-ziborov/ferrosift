use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

pub(super) fn rot13(
    input: &[u8],
    lower: bool,
    upper: bool,
    numbers: bool,
    amount: i128,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if amount == 0 {
        return Ok(input.to_vec());
    }
    let (amount_letters, amount_numbers) = if amount < 0 {
        let abs = amount.unsigned_abs();
        (
            u8::try_from(26 - (abs % 26)).unwrap_or(0),
            u8::try_from(10 - (abs % 10)).unwrap_or(0),
        )
    } else {
        (
            u8::try_from(amount % 26).unwrap_or(0),
            u8::try_from(amount % 10).unwrap_or(0),
        )
    };
    let mut output = input.to_vec();
    for (index, byte) in output.iter_mut().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let value = *byte;
        if upper && value.is_ascii_uppercase() {
            *byte = b'A' + (value - b'A' + amount_letters) % 26;
        } else if lower && value.is_ascii_lowercase() {
            *byte = b'a' + (value - b'a' + amount_letters) % 26;
        } else if numbers && value.is_ascii_digit() {
            *byte = b'0' + (value - b'0' + amount_numbers) % 10;
        }
    }
    context.ensure_active()?;
    Ok(output)
}
