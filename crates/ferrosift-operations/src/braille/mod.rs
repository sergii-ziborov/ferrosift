//! Braille transcription and combining-mark text decoration.

mod codec;
mod operation;

pub use operation::{FromBraille, ToBraille, UnicodeTextFormat};
