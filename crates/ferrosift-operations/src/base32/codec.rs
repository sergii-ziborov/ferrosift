use alloc::{collections::BTreeSet, string::String, vec::Vec};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_ALPHABET: &str = "encoding.base32.invalid_alphabet";
const INVALID_CHARACTER: &str = "encoding.base32.invalid_character";

/// A Base32 alphabet of 32 symbols with an optional trailing padding symbol.
pub(super) struct Alphabet {
    symbols: Vec<char>,
}

impl Alphabet {
    pub(super) fn parse(expression: &str) -> Result<Self, OperationError> {
        let symbols = crate::alphabet::expand(expression, INVALID_ALPHABET)?;
        let mut unique = BTreeSet::new();
        if !matches!(symbols.len(), 32 | 33)
            || !symbols.iter().all(char::is_ascii)
            || !symbols.iter().all(|value| unique.insert(*value))
        {
            return Err(failed(INVALID_ALPHABET));
        }
        Ok(Self { symbols })
    }

    fn symbol(&self, index: i32) -> Option<char> {
        usize::try_from(index)
            .ok()
            .and_then(|index| (index < self.symbols.len()).then(|| self.symbols[index]))
    }

    /// Index of a symbol in the full alphabet, `-1` when absent, mirroring
    /// the JavaScript `String.prototype.indexOf` the reference relies on.
    fn index(&self, symbol: char) -> i32 {
        self.symbols
            .iter()
            .position(|candidate| *candidate == symbol)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    fn contains(&self, symbol: char) -> bool {
        self.symbols.contains(&symbol)
    }
}

pub(super) fn encode(
    input: &[u8],
    alphabet: &Alphabet,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let capacity = input
        .len()
        .div_ceil(5)
        .checked_mul(8)
        .ok_or(OperationError::OutputLimitExceeded)?;
    if u64::try_from(capacity).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }

    let mut output = String::with_capacity(capacity);
    for (index, group) in input.chunks(5).enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        let byte = |offset: usize| i32::from(group.get(offset).copied().unwrap_or(0));
        let (b1, b2, b3, b4, b5) = (byte(0), byte(1), byte(2), byte(3), byte(4));
        let mut encoded = [
            b1 >> 3,
            ((b1 & 7) << 2) | (b2 >> 6),
            (b2 >> 1) & 31,
            ((b2 & 1) << 4) | (b3 >> 4),
            ((b3 & 15) << 1) | (b4 >> 7),
            (b4 >> 2) & 31,
            ((b4 & 3) << 3) | (b5 >> 5),
            b5 & 31,
        ];
        let used = match group.len() {
            1 => 2,
            2 => 4,
            3 => 5,
            4 => 7,
            _ => 8,
        };
        for slot in encoded.iter_mut().skip(used) {
            *slot = 32;
        }
        for value in encoded {
            // The reference drops indexes past the end of short alphabets, so
            // unpadded alphabets emit unpadded text.
            if let Some(symbol) = alphabet.symbol(value) {
                output.push(symbol);
            }
        }
    }
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn decode(
    input: &str,
    alphabet: &Alphabet,
    remove_non_alphabet: bool,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let symbols: Vec<char> = if remove_non_alphabet {
        input
            .chars()
            .filter(|value| alphabet.contains(*value))
            .collect()
    } else {
        input.chars().collect()
    };

    // Absent trailing symbols look up a literal `=`, exactly as the
    // reference substitutes `charAt(i) || "="` before indexing.
    let filler = alphabet.index('=');
    let mut output = Vec::new();
    for (index, group) in symbols.chunks(8).enumerate() {
        if index % 1024 == 0 {
            context.ensure_active()?;
        }
        let encoded = |offset: usize| {
            group
                .get(offset)
                .map_or(filler, |symbol| alphabet.index(*symbol))
        };
        let (e1, e2, e3, e4) = (encoded(0), encoded(1), encoded(2), encoded(3));
        let (e5, e6, e7, e8) = (encoded(4), encoded(5), encoded(6), encoded(7));
        let mut push = |value: i32| -> Result<(), OperationError> {
            if output.len() as u64 >= context.budget().max_output_bytes {
                return Err(OperationError::OutputLimitExceeded);
            }
            output.push(u8::try_from(value).map_err(|_| failed(INVALID_CHARACTER))?);
            Ok(())
        };
        push((e1 << 3) | (e2 >> 2))?;
        if (e2 & 3) != 0 || e3 != 32 {
            push(((e2 & 3) << 6) | (e3 << 1) | (e4 >> 4))?;
        }
        if (e4 & 15) != 0 || e5 != 32 {
            push(((e4 & 15) << 4) | (e5 >> 1))?;
        }
        if (e5 & 1) != 0 || e6 != 32 {
            push(((e5 & 1) << 7) | (e6 << 2) | (e7 >> 3))?;
        }
        if (e7 & 7) != 0 || e8 != 32 {
            push(((e7 & 7) << 5) | e8)?;
        }
    }
    context.ensure_active()?;
    Ok(output)
}
