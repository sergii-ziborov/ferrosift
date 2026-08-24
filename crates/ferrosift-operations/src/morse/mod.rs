//! Morse code, in either direction.

mod codec;
mod operation;
mod table;

pub use operation::{FromMorseCode, ToMorseCode};
