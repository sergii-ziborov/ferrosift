//! Incremental Base64 decode session.

use alloc::boxed::Box;
use alloc::vec::Vec;

use ferrosift_core::{OperationError, StreamSession, StreamSink};

use super::alphabet::{Alphabet, failed};

/// Base64 decode, a chunk at a time.
///
/// Holds at most three pending alphabet symbols between chunks. A complete
/// group of four is decoded and emitted immediately; padding and length rules
/// for a trailing partial group are applied in [`StreamSession::finish`].
pub(crate) struct Base64DecodeSession {
    alphabet: Alphabet,
    remove_non_alphabet: bool,
    pending: Vec<u8>,
    scratch: Vec<u8>,
}

impl Base64DecodeSession {
    pub(crate) fn new(alphabet: Alphabet, remove_non_alphabet: bool) -> Self {
        Self {
            alphabet,
            remove_non_alphabet,
            pending: Vec::with_capacity(4),
            scratch: Vec::new(),
        }
    }

    fn push_symbol(&mut self, symbol: u8, sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        self.pending.push(symbol);
        if self.pending.len() < 4 {
            return Ok(());
        }
        let quad = [
            self.pending[0],
            self.pending[1],
            self.pending[2],
            self.pending[3],
        ];
        self.pending.clear();
        self.emit_group(&quad, sink)
    }

    fn emit_group(
        &mut self,
        group: &[u8],
        sink: &mut dyn StreamSink,
    ) -> Result<(), OperationError> {
        let pad = self.alphabet.padding_byte();
        let data = if let Some(pad_byte) = pad {
            let first_pad = group.iter().position(|byte| *byte == pad_byte);
            if let Some(at) = first_pad {
                if group[at..].iter().any(|byte| *byte != pad_byte) {
                    return Err(failed("encoding.base64.invalid_padding"));
                }
                &group[..at]
            } else {
                group
            }
        } else {
            group
        };
        match data.len() {
            0 => Ok(()),
            1 => Err(failed("encoding.base64.invalid_length")),
            2 => {
                let first = value(data[0], &self.alphabet)?;
                let second = value(data[1], &self.alphabet)?;
                sink.write(&[(first << 2) | (second >> 4)])
            }
            3 => {
                let first = value(data[0], &self.alphabet)?;
                let second = value(data[1], &self.alphabet)?;
                let third = value(data[2], &self.alphabet)?;
                sink.write(&[(first << 2) | (second >> 4), (second << 4) | (third >> 2)])
            }
            4 => {
                let packed = u32::from(value(data[0], &self.alphabet)?) << 18
                    | u32::from(value(data[1], &self.alphabet)?) << 12
                    | u32::from(value(data[2], &self.alphabet)?) << 6
                    | u32::from(value(data[3], &self.alphabet)?);
                sink.write(&[
                    ((packed >> 16) & 0xff) as u8,
                    ((packed >> 8) & 0xff) as u8,
                    (packed & 0xff) as u8,
                ])
            }
            _ => Err(failed("encoding.base64.invalid_length")),
        }
    }
}

impl StreamSession for Base64DecodeSession {
    fn push(&mut self, chunk: &[u8], sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        for &byte in chunk {
            if !self.alphabet.contains_byte(byte) {
                if self.remove_non_alphabet {
                    continue;
                }
                return Err(failed("encoding.base64.invalid_character"));
            }
            self.push_symbol(byte, sink)?;
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>, sink: &mut dyn StreamSink) -> Result<(), OperationError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.pending.len() == 1 {
            return Err(failed("encoding.base64.invalid_length"));
        }
        self.scratch.clear();
        self.scratch.extend_from_slice(&self.pending);
        let pending = core::mem::take(&mut self.scratch);
        self.emit_group(&pending, sink)
    }
}

fn value(symbol: u8, alphabet: &Alphabet) -> Result<u8, OperationError> {
    alphabet
        .value_byte(symbol)
        .ok_or_else(|| failed("encoding.base64.invalid_character"))
}
