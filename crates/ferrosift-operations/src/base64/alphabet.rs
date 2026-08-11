use alloc::{collections::BTreeSet, vec::Vec};

use ferrosift_core::{OperationError, OperationFailureCode};

pub(super) struct Alphabet {
    symbols: Vec<char>,
    padding: Option<char>,
}

impl Alphabet {
    pub(super) fn parse(expression: &str) -> Result<Self, OperationError> {
        let expanded = expand(expression)?;
        if !matches!(expanded.len(), 64 | 65) || !expanded.iter().all(char::is_ascii) {
            return Err(failed("encoding.base64.invalid_alphabet"));
        }
        let symbols = expanded[..64].to_vec();
        let padding = expanded.get(64).copied();
        let mut unique = BTreeSet::new();
        if !symbols.iter().all(|value| unique.insert(*value))
            || padding.is_some_and(|value| !unique.insert(value))
        {
            return Err(failed("encoding.base64.invalid_alphabet"));
        }
        Ok(Self { symbols, padding })
    }

    pub(super) fn symbol(&self, index: usize) -> char {
        self.symbols[index]
    }

    pub(super) const fn padding(&self) -> Option<char> {
        self.padding
    }

    pub(super) fn value(&self, symbol: char) -> Option<u8> {
        self.symbols
            .iter()
            .position(|candidate| *candidate == symbol)
            .and_then(|index| u8::try_from(index).ok())
    }

    pub(super) fn contains(&self, symbol: char) -> bool {
        self.value(symbol).is_some() || self.padding == Some(symbol)
    }
}

fn expand(expression: &str) -> Result<Vec<char>, OperationError> {
    let input: Vec<_> = expression.chars().collect();
    let mut output = Vec::new();
    let mut index = 0;
    while index < input.len() {
        if index + 2 < input.len() && input[index + 1] == '-' && input[index] != '\\' {
            let start = u32::from(input[index]);
            let end = u32::from(input[index + 2]);
            for value in start..=end {
                output.push(
                    char::from_u32(value)
                        .ok_or_else(|| failed("encoding.base64.invalid_alphabet"))?,
                );
            }
            index += 3;
        } else if index + 1 < input.len() && input[index] == '\\' && input[index + 1] == '-' {
            output.push('-');
            index += 2;
        } else {
            output.push(input[index]);
            index += 1;
        }
    }
    Ok(output)
}

pub(super) fn failed(value: &'static str) -> OperationError {
    OperationError::Failed {
        code: OperationFailureCode::from_static(value),
    }
}
