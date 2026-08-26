use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_model::DecimalValue;

/// Hundred-nanosecond intervals between 1601-01-01 and the UNIX epoch.
///
/// Written as a literal because the reference writes it as a literal.
/// Deriving it from a calendar would be a second place to be wrong, and the
/// derivation would have to agree with this number anyway.
pub(crate) const EPOCH_OFFSET: &str = "116444736000000000";

/// The unit names, and how many hundred-nanosecond intervals each one is.
///
/// A nanosecond is *smaller* than the interval, so its entry divides where the
/// others multiply -- and that division is the only inexact step in either
/// direction.
pub(crate) enum Unit {
    /// Multiplied by this many intervals.
    Times(&'static str),
    /// Divided by this many, because the unit is finer than an interval.
    Over(&'static str),
}

/// Resolves a unit name, or `None` for one the reference does not know.
///
/// The micro sign here is U+03BC, the Greek letter, rather than U+00B5, the
/// micro sign proper. They look identical and the reference uses the first, so
/// an argument carrying the second names no unit at all.
pub(crate) fn unit(name: &str) -> Option<Unit> {
    match name {
        "Seconds (s)" => Some(Unit::Times("10000000")),
        "Milliseconds (ms)" => Some(Unit::Times("10000")),
        "Microseconds (\u{03bc}s)" => Some(Unit::Times("10")),
        "Nanoseconds (ns)" => Some(Unit::Over("100")),
        _ => None,
    }
}

/// A scale factor as a value.
pub(crate) fn factor(text: &str) -> DecimalValue {
    DecimalValue::parse(text)
}

/// Reverses a hex string two characters at a time, on the way out.
///
/// Transcribed rather than tidied, because an odd-length string is where the
/// two directions stop being inverses. This one reverses the pairs it can and
/// then appends a `0` and the *first* character, so `abcde` becomes `debc0a`;
/// [`flip_back`] moves the last character to the front instead. Writing either
/// one "properly" would break the round trip the corpus pins.
///
/// Indexed by character where the reference indexes by UTF-16 code unit. The
/// two agree for every string that reaches here: this one only ever sees the
/// output of a base-sixteen rendering, which is ASCII.
pub(crate) fn flip_forward(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut flipped = String::with_capacity(text.len() + 2);

    let mut index = characters.len().checked_sub(2);
    while let Some(at) = index {
        flipped.push(characters[at]);
        if let Some(next) = characters.get(at + 1) {
            flipped.push(*next);
        }
        index = at.checked_sub(2);
    }
    if !characters.len().is_multiple_of(2) {
        flipped.push('0');
        if let Some(first) = characters.first() {
            flipped.push(*first);
        }
    }
    flipped
}

/// Reverses a hex string two characters at a time, on the way in.
///
/// The counterpart to [`flip_forward`], and not its inverse for an odd-length
/// string: this one moves the trailing character to the front and then
/// reverses the even prefix.
///
/// Indexed by character where the reference indexes by UTF-16 code unit. The
/// difference is unobservable: this only permutes characters, so text that was
/// not hexadecimal before is not hexadecimal after, and the reading that
/// follows refuses it either way.
pub(crate) fn flip_back(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let length = characters.len();
    let mut result = String::with_capacity(text.len() + 1);

    if !length.is_multiple_of(2)
        && let Some(last) = characters.last()
    {
        result.push(*last);
    }
    let mut index = (length - length % 2).checked_sub(2);
    while let Some(at) = index {
        result.push(characters[at]);
        if let Some(next) = characters.get(at + 1) {
            result.push(*next);
        }
        index = at.checked_sub(2);
    }
    result
}
