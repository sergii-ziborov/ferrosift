use ferrosift_core::OperationError;

pub(super) use crate::failure::failed;

pub(super) struct Alphabet {
    padding: Option<char>,
    /// Reverse lookup, indexed by ASCII code point.
    ///
    /// The alphabet is validated as ASCII at parse time, so 128 slots cover
    /// every symbol it can hold. Searching the symbol list instead cost a
    /// 64-element scan per character, several times per character, which is
    /// what made decoding two orders of magnitude slower than it needed to be.
    values: [Option<u8>; 128],
    /// The same symbols as bytes.
    ///
    /// Every symbol is ASCII by the same validation, so each is exactly one
    /// byte. Emitting bytes lets the encoder build a `Vec<u8>` directly and
    /// skip the UTF-8 encode that `String::push` runs for every character.
    symbol_bytes: [u8; 64],
    padding_byte: Option<u8>,
}

impl Alphabet {
    pub(super) fn parse(expression: &str) -> Result<Self, OperationError> {
        let expanded = crate::alphabet::expand(expression, "encoding.base64.invalid_alphabet")?;
        if !matches!(expanded.len(), 64 | 65) || !expanded.iter().all(char::is_ascii) {
            return Err(failed("encoding.base64.invalid_alphabet"));
        }
        let symbols = &expanded[..64];
        let padding = expanded.get(64).copied();

        // Uniqueness on a stack bitmap rather than a `BTreeSet`. Every symbol
        // is ASCII by the check above, so 128 flags decide it exactly. This
        // runs on every call, where the set's per-node allocations cost more
        // than encoding a short input does.
        let mut seen = [false; 128];
        let mut values = [None; 128];
        let mut symbol_bytes = [0u8; 64];
        for (index, symbol) in symbols.iter().enumerate() {
            let slot = *symbol as usize;
            if core::mem::replace(&mut seen[slot], true) {
                return Err(failed("encoding.base64.invalid_alphabet"));
            }
            values[slot] = u8::try_from(index).ok();
            symbol_bytes[index] = u8::try_from(u32::from(*symbol)).unwrap_or(0);
        }
        if let Some(value) = padding
            && core::mem::replace(&mut seen[value as usize], true)
        {
            return Err(failed("encoding.base64.invalid_alphabet"));
        }

        Ok(Self {
            padding,
            values,
            symbol_bytes,
            padding_byte: padding.map(|value| u8::try_from(u32::from(value)).unwrap_or(0)),
        })
    }

    /// The symbol for an index, as a byte.
    pub(super) const fn symbol_byte(&self, index: usize) -> u8 {
        self.symbol_bytes[index]
    }

    /// The padding symbol, as a byte.
    pub(super) const fn padding_byte(&self) -> Option<u8> {
        self.padding_byte
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
