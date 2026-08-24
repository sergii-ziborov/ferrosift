//! Affine, Atbash, and Vigenère: substitutions over the 26-letter alphabet.

use alloc::{string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

use super::letters::{
    from_units, gcd, lower_index, lower_unit, lowered_index, mod_inverse, units, upper_unit,
};

const NOT_INTEGER: &str = "cipher.affine.non_integer";
const NOT_COPRIME: &str = "cipher.affine.not_coprime";
const EMPTY_KEY: &str = "cipher.vigenere.empty_key";
const NON_LETTER_KEY: &str = "cipher.vigenere.non_letter_key";

/// Validates the affine parameters the way the reference does.
///
/// Both must be non-negative integers — the reference tests them against
/// `/^\+?(0|[1-9]\d*)$/`, which rejects a negative `a` or `b` outright rather
/// than reducing it modulo 26 — and `a` must be coprime to 26.
fn check_affine(a: i128, b: i128) -> Result<(), OperationError> {
    if a < 0 || b < 0 {
        return Err(failed(NOT_INTEGER));
    }
    if gcd(a, 26) != 1 {
        return Err(failed(NOT_COPRIME));
    }
    Ok(())
}

/// Applies a per-letter substitution, preserving case and passing everything
/// else through untouched.
fn substitute<F>(
    input: &str,
    context: &OperationContext<'_>,
    mut map: F,
) -> Result<String, OperationError>
where
    F: FnMut(usize) -> usize,
{
    let source = units(input);
    let mut output: Vec<u16> = Vec::with_capacity(source.len());
    for (position, unit) in source.iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        if let Some(index) = lower_index(*unit) {
            output.push(lower_unit(map(index)));
        } else if let Some(index) = lowered_index(*unit) {
            output.push(upper_unit(map(index)));
        } else {
            output.push(*unit);
        }
    }
    Ok(from_units(&output))
}

/// `ax + b (mod 26)`.
pub(super) fn affine_encode(
    input: &str,
    a: i128,
    b: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    check_affine(a, b)?;
    substitute(input, context, |index| {
        usize::try_from((a * index as i128 + b).rem_euclid(26)).unwrap_or(0)
    })
}

/// `(y - b) * a⁻¹ (mod 26)`.
pub(super) fn affine_decode(
    input: &str,
    a: i128,
    b: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    check_affine(a, b)?;
    let inverse = mod_inverse(a, 26).ok_or_else(|| failed(NOT_COPRIME))?;
    substitute(input, context, |index| {
        usize::try_from(((index as i128 - b) * inverse).rem_euclid(26)).unwrap_or(0)
    })
}

/// Which direction `vigenere` shifts by the key.
#[derive(Clone, Copy)]
pub(super) enum Shift {
    Forward,
    Backward,
}

/// Which direction the affine substitution runs.
#[derive(Clone, Copy)]
pub(super) enum Direction {
    Encode,
    Decode,
}

/// Vigenère in either direction.
///
/// The key advances only on letters: the reference counts skipped units in
/// `fail` and indexes the key by `i - fail`, so punctuation and spaces do not
/// consume key material. Because that count is in UTF-16 units, one astral
/// character advances it by two.
pub(super) fn vigenere(
    input: &str,
    key: &str,
    shift: Shift,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let key_units = units(key);
    if key_units.is_empty() {
        return Err(failed(EMPTY_KEY));
    }
    let key_indices: Vec<usize> = key_units
        .iter()
        .map(|unit| lowered_index(*unit).ok_or_else(|| failed(NON_LETTER_KEY)))
        .collect::<Result<_, _>>()?;

    let source = units(input);
    let mut output: Vec<u16> = Vec::with_capacity(source.len());
    let mut skipped = 0usize;
    for (position, unit) in source.iter().enumerate() {
        if position.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let key_index = key_indices[(position - skipped) % key_indices.len()];
        let shifted = |index: usize| match shift {
            Shift::Forward => (index + key_index) % 26,
            Shift::Backward => (index + 26 - key_index) % 26,
        };
        if let Some(index) = lower_index(*unit) {
            output.push(lower_unit(shifted(index)));
        } else if let Some(index) = lowered_index(*unit) {
            output.push(upper_unit(shifted(index)));
        } else {
            output.push(*unit);
            skipped += 1;
        }
    }
    context.ensure_active()?;
    Ok(from_units(&output))
}

/// Atbash is the affine cipher with both parameters at 25.
pub(super) fn atbash(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    affine_encode(input, 25, 25, context)
}
