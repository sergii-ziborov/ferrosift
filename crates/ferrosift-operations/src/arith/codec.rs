use alloc::vec::Vec;

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

/// Folds a list left to right, or `None` for an empty one.
///
/// The reference reduces *without a seed*, and that is not a detail. A seed of
/// zero would be right for a total and wrong for a difference, where the first
/// item is not an operand but the starting point: `10 3 2` is five, not
/// negative fifteen. A one-item list answers that item untouched, and an empty
/// one answers nothing at all -- which every operation here turns into
/// not-a-number, exactly as the reference does.
fn fold(
    values: &[DecimalValue],
    step: fn(&DecimalValue, &DecimalValue) -> DecimalValue,
) -> Option<DecimalValue> {
    let (first, rest) = values.split_first()?;
    let mut total = first.clone();
    for value in rest {
        total = step(&total, value);
    }
    Some(total)
}

/// The total of a list.
pub(crate) fn total(values: &[DecimalValue]) -> Option<DecimalValue> {
    fold(values, bignumber::plus)
}

/// The first value less every value after it.
pub(crate) fn difference(values: &[DecimalValue]) -> Option<DecimalValue> {
    fold(values, bignumber::minus)
}

/// The product of a list.
pub(crate) fn product(values: &[DecimalValue]) -> Option<DecimalValue> {
    fold(values, bignumber::times)
}

/// The first value divided by every value after it, in order.
pub(crate) fn quotient(values: &[DecimalValue]) -> Option<DecimalValue> {
    fold(values, bignumber::divide)
}

/// The mean, or `None` for an empty list.
pub(crate) fn average(values: &[DecimalValue]) -> Option<DecimalValue> {
    if values.is_empty() {
        return None;
    }
    Some(bignumber::mean(values))
}

/// The middle value, or the mean of the middle two.
pub(crate) fn middle(values: &[DecimalValue]) -> Option<DecimalValue> {
    if values.is_empty() {
        return None;
    }
    Some(bignumber::median(values))
}

/// The standard deviation of a population, in the order the reference takes it.
///
/// The order is load-bearing. Each squared deviation is exact, and their sum
/// is exact, but the division by the count rounds and the root rounds again --
/// so the root is taken of an *already rounded* quotient. Rounding once at the
/// end instead would agree on most inputs and disagree on the ones where the
/// twentieth place is close, which is the hardest kind of difference to spot.
pub(crate) fn standard_deviation(values: &[DecimalValue]) -> Option<DecimalValue> {
    let average = average(values)?;
    let mut squares = DecimalValue::zero();
    for value in values {
        let spread = bignumber::minus(value, &average);
        squares = bignumber::plus(&squares, &bignumber::times(&spread, &spread));
    }
    let count = DecimalValue::from(i128::try_from(values.len()).unwrap_or(i128::MAX));
    Some(bignumber::square_root(&bignumber::divide(&squares, &count)))
}
