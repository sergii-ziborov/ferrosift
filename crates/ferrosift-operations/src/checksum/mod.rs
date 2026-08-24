//! Checksums: Adler-32, Fletcher at four widths, TCP/IP, XOR, and Luhn.

mod codec;
mod operation;

pub use operation::{Checksum, LuhnChecksum};
