use alloc::{collections::BTreeSet, vec::Vec};

use ferrosift_core::OperationError;

pub(super) use crate::failure::failed;

pub(super) struct Alphabet {
    symbols: Vec<char>,
    padding: Option<char>,
    /// Reverse lookup, indexed by ASCII code point.
    ///
    /// The alphabet is validated as ASCII at parse time, so 128 slots cover
    /// every symbol it can hold. Searching the symbol list instead cost a
    /// 64-element scan per character, several times per character, which is
    /// what made decoding two orders of magnitude slower than it needed to be.
    values: [Option<u8>; 128],
}

impl Alphabet {
    pub(super) fn parse(expression: &str) -> Result<Self, OperationError> {
        let expanded = crate::alphabet::expand(expression, "encoding.base64.invalid_alphabet")?;
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
        let mut values = [None; 128];
        for (index, symbol) in symbols.iter().enumerate() {
            values[*symbol as usize] = u8::try_from(index).ok();
        }
        Ok(Self {
            symbols,
            padding,
            values,
        })
    }

    pub(super) fn symbol(&self, index: usize) -> char {
        self.symbols[index]
    }

    pub(super) const fn padding(&self) -> Option<char> {
        self.padding
    }

    pub(super) fn value(&self, symbol: char) -> Option<u8> {
        self.values.get(symbol as usize).copied().flatten()
    }

    pub(super) fn contains(&self, symbol: char) -> bool {
        self.value(symbol).is_some() || self.padding == Some(symbol)
    }
}
