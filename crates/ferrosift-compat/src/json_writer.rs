//! Bounded compact JSON output.

use alloc::string::String;

use crate::{error::ExportError, profile::MAX_RECIPE_BYTES};

pub(crate) struct CappedJson {
    output: String,
    limit: usize,
}

impl CappedJson {
    pub(crate) const fn new() -> Self {
        Self {
            output: String::new(),
            limit: MAX_RECIPE_BYTES,
        }
    }

    #[cfg(test)]
    const fn with_limit(limit: usize) -> Self {
        Self {
            output: String::new(),
            limit,
        }
    }

    pub(crate) fn push_raw(&mut self, value: &str) -> Result<(), ExportError> {
        self.ensure_capacity(value.len())?;
        self.output.push_str(value);
        Ok(())
    }

    pub(crate) fn push_string(&mut self, value: &str) -> Result<(), ExportError> {
        self.push_raw("\"")?;
        for character in value.chars() {
            match character {
                '"' => self.push_raw("\\\"")?,
                '\\' => self.push_raw("\\\\")?,
                '\u{0008}' => self.push_raw("\\b")?,
                '\t' => self.push_raw("\\t")?,
                '\n' => self.push_raw("\\n")?,
                '\u{000c}' => self.push_raw("\\f")?,
                '\r' => self.push_raw("\\r")?,
                '\u{0000}'..='\u{001f}' => self.push_control_escape(character)?,
                _ => self.push_char(character)?,
            }
        }
        self.push_raw("\"")
    }

    pub(crate) fn finish(self) -> String {
        self.output
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.output.len()
    }

    fn push_control_escape(&mut self, character: char) -> Result<(), ExportError> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let value = character as usize;
        self.push_raw("\\u00")?;
        self.push_char(char::from(HEX[value >> 4]))?;
        self.push_char(char::from(HEX[value & 0x0f]))
    }

    fn push_char(&mut self, character: char) -> Result<(), ExportError> {
        self.ensure_capacity(character.len_utf8())?;
        self.output.push(character);
        Ok(())
    }

    fn ensure_capacity(&self, additional: usize) -> Result<(), ExportError> {
        if self
            .output
            .len()
            .checked_add(additional)
            .is_some_and(|length| length <= self.limit)
        {
            Ok(())
        } else {
            Err(ExportError::RecipeTooLarge)
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::CappedJson;
    use crate::error::ExportError;

    #[test]
    fn oversized_string_never_grows_output_beyond_the_cap() {
        let mut writer = CappedJson::with_limit(32);
        let source = core::iter::repeat_n('x', 64).collect::<String>();

        let error = writer
            .push_string(&source)
            .expect_err("escaped output crosses the cap");

        assert_eq!(error, ExportError::RecipeTooLarge);
        assert!(writer.len() <= 32);
    }

    #[test]
    fn strings_use_compact_json_escaping() {
        let mut writer = CappedJson::with_limit(128);

        writer
            .push_string("line\n\"\\\u{0001}é")
            .expect("escaped string fits");

        assert_eq!(writer.finish(), "\"line\\n\\\"\\\\\\u0001é\"");
    }
}
