//! Type-Length-Value, and the JSON the reference renders it as.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};
use ferrosift_model::StructuredValue;

use crate::failure::failed;

/// One parsed record.
struct Record {
    /// Absent when the key size is zero, which drops the field entirely.
    key: Option<Vec<Option<u8>>>,
    /// Not a number when a length byte was read past the end.
    length: f64,
    value: Vec<Option<u8>>,
}

/// Walks the input, tracking how far in it has read.
struct Reader<'a> {
    input: &'a [u8],
    location: usize,
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, location: 0 }
    }

    /// Whether the cursor has reached or passed the end.
    fn at_end(&self) -> bool {
        self.input.len() <= self.location
    }

    /// The byte at the cursor, or `None` when the cursor is past the end.
    ///
    /// Reading past the end is not an error in the reference -- indexing a
    /// typed array out of range yields `undefined`, which then travels through
    /// the arithmetic and into the output rather than stopping the parse.
    fn peek(&self) -> Option<u8> {
        self.input.get(self.location).copied()
    }

    /// Reads a length, in whichever of the two encodings is selected.
    ///
    /// A byte read past the end makes the running total not-a-number, because
    /// the reference multiplies `undefined` into it. Every later use of that
    /// length then behaves as the reference's `NaN` does, which is to produce
    /// nothing rather than to fail.
    fn length(&mut self, bytes: i64, ber: bool) -> f64 {
        let mut bytes = bytes;
        let mut big_endian = false;

        if ber {
            let first = self.peek();
            self.location += 1;
            match first {
                // The high bit says the byte is a count of further length
                // bytes rather than the length itself.
                Some(value) if value & 0x80 != 0 => {
                    bytes = i64::from(value & 0x7f);
                    big_endian = true;
                }
                Some(value) => return f64::from(value & 0x7f),
                // Past the end, `undefined & 0x80` is zero and the masked
                // value is zero too.
                None => return 0.0,
            }
        }

        let mut length = 0.0_f64;
        let mut index = 0_i64;
        while index < bytes {
            let byte = self.peek();
            self.location += 1;
            index += 1;
            match byte {
                Some(value) if big_endian => {
                    length = length * 256.0 + f64::from(value);
                }
                Some(value) => {
                    // Little-endian, written as a scaled sum rather than a
                    // shift: the reference uses `Math.pow`, which keeps going
                    // past the width a shift would wrap at.
                    length += f64::from(value) * power_of_256(index - 1);
                }
                None => return f64::NAN,
            }
        }
        length
    }

    /// Reads `length` bytes, stopping once the cursor is *past* the end.
    ///
    /// The test is strictly greater, so the byte at exactly the end is still
    /// read -- and it is `undefined`, which becomes a `null` in the output.
    /// Exactly one of those can appear, because the next test then stops.
    fn value(&mut self, length: f64) -> Vec<Option<u8>> {
        let mut collected = Vec::new();
        if length.is_nan() || length <= 0.0 {
            return collected;
        }
        let mut index = 0.0_f64;
        while index < length {
            if self.location > self.input.len() {
                return collected;
            }
            collected.push(self.peek());
            self.location += 1;
            index += 1.0;
        }
        collected
    }
}

/// Widens a byte count to the type the reference does its arithmetic in.
///
/// Every value that reaches this is a field size or an index bounded by the
/// input length, so the narrowing at 2^53 is unreachable in practice and the
/// widening is exact.
#[expect(
    clippy::cast_precision_loss,
    reason = "the argument is a byte count, far below the mantissa's range"
)]
fn widen(count: i64) -> f64 {
    count as f64
}

/// 256 raised to `exponent`, without leaving `core`.
fn power_of_256(exponent: i64) -> f64 {
    let mut result = 1.0_f64;
    let mut remaining = exponent;
    while remaining > 0 {
        result *= 256.0;
        remaining -= 1;
    }
    result
}

/// Parses the input into the structure the reference's JSON dish holds.
pub(super) fn parse(
    input: &[u8],
    key_size: i64,
    length_size: i64,
    ber: bool,
    context: &OperationContext<'_>,
) -> Result<StructuredValue, OperationError> {
    context.ensure_active()?;
    if key_size <= 0 && length_size <= 0 {
        return Err(failed("parsing.tlv.no_field_size"));
    }

    let mut reader = Reader::new(input);
    let mut records = Vec::new();
    while !reader.at_end() {
        context.ensure_active()?;
        let before = reader.location;
        // The key size is a count of bytes, so it is small enough that the
        // widening is exact; the reference holds it as a number throughout,
        // which is what `value` is written against.
        let key = if key_size > 0 {
            Some(reader.value(widen(key_size)))
        } else {
            None
        };
        let length = reader.length(length_size, ber);
        let value = reader.value(length);
        records.push(Record { key, length, value });

        // Without this the loop cannot end on input that consumes nothing --
        // a zero key size with BER off and a zero length reads no bytes at
        // all, and the reference would spin forever where this stops.
        if reader.location == before {
            break;
        }
    }

    Ok(StructuredValue::List(
        records.into_iter().map(structure).collect(),
    ))
}

/// One record as the object the reference builds.
///
/// An absent key is left out rather than written as null: the reference sets
/// the property to `undefined`, and `JSON.stringify` drops such a property
/// instead of emitting it. A length that overran the input is `NaN` there,
/// which has no JSON spelling and is written as null -- so it is null here.
fn structure(record: Record) -> StructuredValue {
    // Pushed in the order the reference sets them, which is now the order they
    // are written: a sorted map used to make this agree by luck, because
    // `key`, `length`, `value` happens to sort the way it is built.
    let mut entries = Vec::new();
    if let Some(key) = record.key {
        entries.push((String::from("key"), bytes(&key)));
    }
    entries.push((String::from("length"), narrow(record.length)));
    entries.push((String::from("value"), bytes(&record.value)));
    StructuredValue::Object(entries)
}

/// A length as the structure holds it, or null where the reference had `NaN`.
///
/// `JSON.stringify` has no spelling for `NaN` and writes null, so a length
/// that overran the input is reported as no length at all. The narrowing is
/// reached only for a finite value, and a length is bounded by the input.
#[expect(
    clippy::cast_possible_truncation,
    reason = "only finite lengths reach the cast, and a length is bounded by the input"
)]
fn narrow(length: f64) -> StructuredValue {
    if length.is_nan() || length.is_infinite() {
        return StructuredValue::Null;
    }
    StructuredValue::Integer(length as i128)
}

/// A byte list, where a byte read past the end is null.
fn bytes(values: &[Option<u8>]) -> StructuredValue {
    StructuredValue::List(
        values
            .iter()
            .map(|value| match value {
                Some(byte) => StructuredValue::Integer(i128::from(*byte)),
                None => StructuredValue::Null,
            })
            .collect(),
    )
}
