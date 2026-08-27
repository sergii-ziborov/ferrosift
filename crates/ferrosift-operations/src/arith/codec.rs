use alloc::vec::Vec;

use ferrosift_core::OperationError;
use ferrosift_model::DecimalValue;

use crate::jscompat::bignumber;

/// Reads a delimited list of numbers, dropping every token that is not one.
///
/// The reference's `createNumArray` builds a number from each token and keeps
/// it only if it reads as one, so `0x0a 8 .5 apples` is three numbers rather
/// than an error. Both halves of that are load-bearing: the hexadecimal prefix
/// is read, and the word is dropped in silence rather than reported.
///
/// An empty token is not a number either, so a trailing delimiter costs
/// nothing -- which is why `1 2 3 ` and `1 2 3` come to the same total.
pub(crate) fn read_list(input: &str, delimiter: &str) -> Vec<DecimalValue> {
    input
        .split(delimiter)
        .map(DecimalValue::parse)
        .filter(|value| !value.is_not_a_number())
        .collect()
}

/// One reduction step, and what can be said about the size of its answer
/// before taking it.
///
/// Only addition and subtraction carry a floor, because only they can turn two
/// short numbers into a long one: they bring both operands to the finer of the
/// two exponents first, so a wide gap between the exponents *is* the answer's
/// width. Multiplication adds the exponents instead and divides the work by
/// nothing, and division already refuses an out-of-range scale before it
/// computes any digits.
struct Step {
    apply: fn(&DecimalValue, &DecimalValue) -> DecimalValue,
    floor: fn(&DecimalValue, &DecimalValue) -> u64,
}

/// The floor for a step that cannot amplify, which is that nothing is claimed.
fn no_floor(_: &DecimalValue, _: &DecimalValue) -> u64 {
    0
}

/// Folds a list left to right, or `None` for an empty one.
///
/// The reference reduces *without a seed*, and that is not a detail. A seed of
/// zero would be right for a total and wrong for a difference, where the first
/// item is not an operand but the starting point: `10 3 2` is five, not
/// negative fifteen. A one-item list answers that item untouched, and an empty
/// one answers nothing at all -- which every operation here turns into
/// not-a-number, exactly as the reference does.
///
/// `ceiling` bounds every step, not only the last one. That is deliberate and
/// it is narrower than the executor alone would be: `1e10000000 + 1e100 -
/// 1e10000000` has a short answer and a twenty-million-digit middle, and this
/// refuses it. The budget is a `FerroSift` concept the reference does not have
/// at all, and an intermediate nobody could hold is exactly the resource it
/// exists to bound -- a recipe that genuinely wants one can say so by raising
/// `max_output_bytes`.
fn fold(
    values: &[DecimalValue],
    step: &Step,
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    let Some((first, rest)) = values.split_first() else {
        return Ok(None);
    };
    let mut total = first.clone();
    for value in rest {
        if (step.floor)(&total, value) > ceiling {
            return Err(OperationError::OutputLimitExceeded);
        }
        total = (step.apply)(&total, value);
    }
    Ok(Some(total))
}

/// The total of a list.
pub(crate) fn total(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    fold(
        values,
        &Step {
            apply: bignumber::plus,
            floor: bignumber::sum_min_len,
        },
        ceiling,
    )
}

/// The first value less every value after it.
pub(crate) fn difference(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    fold(
        values,
        &Step {
            apply: bignumber::minus,
            floor: bignumber::sum_min_len,
        },
        ceiling,
    )
}

/// The product of a list.
pub(crate) fn product(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    fold(
        values,
        &Step {
            apply: bignumber::times,
            floor: no_floor,
        },
        ceiling,
    )
}

/// The first value divided by every value after it, in order.
pub(crate) fn quotient(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    fold(
        values,
        &Step {
            apply: bignumber::divide,
            floor: no_floor,
        },
        ceiling,
    )
}

/// The mean, or `None` for an empty list.
///
/// The sum is taken through the guarded fold rather than through
/// `bignumber::mean`, which is the same arithmetic reached differently: that
/// one seeds with zero and this one with the first item, and exact addition
/// makes those the same total. What it buys is the ceiling — a mean is a sum
/// before it is a division, and the sum is where the width comes from.
pub(crate) fn average(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    let Some(summed) = total(values, ceiling)? else {
        return Ok(None);
    };
    Ok(Some(bignumber::divide(&summed, &count_of(values))))
}

/// The list's length as a value to divide by.
fn count_of(values: &[DecimalValue]) -> DecimalValue {
    DecimalValue::from(i128::try_from(values.len()).unwrap_or(i128::MAX))
}

/// The middle value, or the mean of the middle two.
pub(crate) fn middle(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    if values.is_empty() {
        return Ok(None);
    }
    // A median is a sort and, for an even list, one mean of the middle pair.
    // The sort no longer costs anything to reach -- `compare` settles two
    // values by their scales before it settles them by their digits -- but the
    // mean is a sum like any other, so it goes through the guarded one.
    let sorted = bignumber::ordered(values);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        return Ok(Some(sorted[middle].clone()));
    }
    average(&sorted[middle - 1..=middle], ceiling)
}

/// The standard deviation of a population, in the order the reference takes it.
///
/// The order is load-bearing. Each squared deviation is exact, and their sum
/// is exact, but the division by the count rounds and the root rounds again --
/// so the root is taken of an *already rounded* quotient. Rounding once at the
/// end instead would agree on most inputs and disagree on the ones where the
/// twentieth place is close, which is the hardest kind of difference to spot.
pub(crate) fn standard_deviation(
    values: &[DecimalValue],
    ceiling: u64,
) -> Result<Option<DecimalValue>, OperationError> {
    let Some(average) = average(values, ceiling)? else {
        return Ok(None);
    };
    let mut squares = DecimalValue::zero();
    for value in values {
        // Both additions are guarded: the deviation subtracts a mean that can
        // sit anywhere relative to the value, and the running total then adds a
        // square that can sit anywhere relative to it.
        if bignumber::sum_min_len(value, &average) > ceiling {
            return Err(OperationError::OutputLimitExceeded);
        }
        let spread = bignumber::minus(value, &average);
        let square = bignumber::times(&spread, &spread);
        if bignumber::sum_min_len(&squares, &square) > ceiling {
            return Err(OperationError::OutputLimitExceeded);
        }
        squares = bignumber::plus(&squares, &square);
    }
    Ok(Some(bignumber::square_root(&bignumber::divide(
        &squares,
        &count_of(values),
    ))))
}
