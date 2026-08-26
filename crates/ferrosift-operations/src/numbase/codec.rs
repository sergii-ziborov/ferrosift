use alloc::string::String;

use ferrosift_model::DecimalValue;

use crate::jscompat::bignumber;
use crate::jscompat::delim::is_js_whitespace;

/// The bases the reference will read or write.
pub(crate) const RADIX_RANGE: core::ops::RangeInclusive<i128> = 2..=36;

/// Reads a number written in `radix`, the way the From Base operation does.
///
/// Deliberately not the constructor with a base, because the reference does
/// not use it that way. It strips whitespace from *everywhere* rather than
/// only from the ends, splits the text on the point itself, and reads each
/// fractional digit on its own. Three consequences follow, and all three are
/// pinned by the corpus:
///
/// - a value whose letters mix case is read here and refused by the
///   constructor, because each digit is read alone and a single digit has no
///   case to disagree with;
/// - a fraction rounds once per digit, where the constructor rounds once for
///   the whole fraction;
/// - a second point is ignored along with everything after it, because the
///   reference indexes the first two pieces and looks no further.
pub(crate) fn from_base(input: &str, radix: u32) -> Option<DecimalValue> {
    let stripped: String = input
        .chars()
        .filter(|character| !is_js_whitespace(*character))
        .collect();

    let mut pieces = stripped.split('.');
    let whole = pieces.next().unwrap_or_default();
    let mut result = bignumber::parse_in_base(whole, radix)?;

    let Some(fraction) = pieces.next() else {
        return Some(result);
    };
    for (index, digit) in fraction.chars().enumerate() {
        let mut buffer = [0_u8; 4];
        let value = bignumber::parse_in_base(digit.encode_utf8(&mut buffer), radix)?;
        let place = index as u64 + 1;
        result = bignumber::plus(
            &result,
            &bignumber::divide(&value, &bignumber::power_of(radix, place)),
        );
    }
    Some(result)
}

/// Writes a number in `radix`, which is the reference's `toString(base)`.
pub(crate) fn to_base(value: &DecimalValue, radix: u32) -> Option<String> {
    bignumber::to_base(value, radix)
}
