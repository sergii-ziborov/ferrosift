use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The reference's own ceiling on how much this operation may produce.
///
/// This is what bounds a generator in place of an expansion ratio. It is the
/// reference's number rather than a `FerroSift` choice, so the two refuse the
/// same requests.
const MAX_PERMUTATIONS: i128 = 50_000;

/// Generates a De Bruijn sequence over `k` symbols with subsequences of `n`.
///
/// The construction is the standard recursive Lyndon-word algorithm the
/// reference uses. Recursion depth is bounded by the key length, and the
/// permutation ceiling above caps that well inside any stack — a nine-symbol
/// alphabet reaches the limit at a key length of five.
///
/// # Errors
///
/// Returns an error outside the reference's documented ranges: `k` from two to
/// nine, `n` at least two, and `k^n` below fifty thousand. The output budget is
/// checked as well, because declaring `InputIndependent` waives the expansion
/// ratio and nothing else.
pub(super) fn de_bruijn(
    k: i128,
    n: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    if !(2..=9).contains(&k) {
        return Err(failed("text.debruijn.invalid_alphabet_size"));
    }
    if n < 2 {
        return Err(failed("text.debruijn.invalid_key_length"));
    }
    let exponent = u32::try_from(n).map_err(|_| failed("text.debruijn.invalid_key_length"))?;
    let permutations = k
        .checked_pow(exponent)
        .ok_or_else(|| failed("text.debruijn.too_many_permutations"))?;
    if permutations > MAX_PERMUTATIONS {
        return Err(failed("text.debruijn.too_many_permutations"));
    }
    // The sequence is exactly k^n symbols long, so the output size is known
    // before anything is built. A generator that skipped this would have no
    // bound at all.
    let size = u64::try_from(permutations).map_err(|_| OperationError::OutputLimitExceeded)?;
    if size > context.budget().max_output_bytes {
        return Err(OperationError::OutputLimitExceeded);
    }

    let k = usize::try_from(k).map_err(|_| failed("text.debruijn.invalid_alphabet_size"))?;
    let n = usize::try_from(n).map_err(|_| failed("text.debruijn.invalid_key_length"))?;

    let mut register = alloc::vec![0usize; k * n + 1];
    let mut sequence: Vec<usize> =
        Vec::with_capacity(usize::try_from(size).map_err(|_| OperationError::OutputLimitExceeded)?);
    generate(1, 1, k, n, &mut register, &mut sequence, context)?;

    context.ensure_active()?;
    Ok(sequence
        .iter()
        .map(|digit| char::from(b'0' + u8::try_from(*digit).unwrap_or(0)))
        .collect())
}

fn generate(
    t: usize,
    p: usize,
    k: usize,
    n: usize,
    register: &mut Vec<usize>,
    sequence: &mut Vec<usize>,
    context: &OperationContext<'_>,
) -> Result<(), OperationError> {
    if t > n {
        if !n.is_multiple_of(p) {
            return Ok(());
        }
        context.ensure_active()?;
        // The reference pushes `a[1..=p]`, so the slice starts at one.
        sequence.extend_from_slice(&register[1..=p]);
        return Ok(());
    }
    register[t] = register[t - p];
    generate(t + 1, p, k, n, register, sequence, context)?;
    for value in register[t - p] + 1..k {
        register[t] = value;
        generate(t + 1, t, k, n, register, sequence, context)?;
    }
    Ok(())
}
