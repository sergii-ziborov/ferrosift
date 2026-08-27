//! Text as one large integer, and back.
//!
//! The text is read as a big-endian run of character codes: `ABC` is
//! `0x414243`, which is `4276803`. That is arbitrary precision by construction
//! -- a sixteen-character string is a hundred-and-twenty-eight-bit number --
//! which is why this waited on `num-bigint` rather than on anything about
//! decimals.
//!
//! The operation guesses what its input is rather than being told: digits are
//! a decimal, `0x` and hex digits are a hexadecimal, quotes mean the text
//! inside them, and anything else is text as written. So `123` is a number and
//! `"123"` is three characters, and the two answer differently.

mod codec;
mod operation;

pub use operation::TextIntegerConversion;
