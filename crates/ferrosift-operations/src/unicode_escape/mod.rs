//! Unicode escapes (`\uXXXX`, `%uXXXX`, `U+XXXX`) and `NetBIOS` name coding.

mod codec;
mod operation;

pub use operation::{
    DecodeNetbiosName, EncodeNetbiosName, EscapeUnicodeCharacters, UnescapeUnicodeCharacters,
};
