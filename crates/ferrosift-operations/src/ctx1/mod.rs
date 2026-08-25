//! Citrix CTX1 password obfuscation.
//!
//! Worth being plain about what this is: an xor chain against a fixed constant
//! with no key, written out as letters. It is reversible by anyone who knows
//! the format, which is why both directions ship together — a CTX1 string in a
//! configuration file is a password in plain sight, and the point of the
//! decoder is to make that obvious rather than to keep it hidden.
//!
//! The decoder reads its input backwards. The chain runs forwards, so
//! recovering one byte needs the *encoded* pair that follows it, which is
//! easier to express by reversing than by looking ahead.

mod codec;
mod operation;

pub use operation::{CitrixCtx1Decode, CitrixCtx1Encode};
