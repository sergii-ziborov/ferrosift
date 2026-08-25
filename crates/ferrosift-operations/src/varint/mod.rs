//! `VarInt` coding and quoted-printable decoding.

mod codec;
mod operation;

pub use operation::{FromQuotedPrintable, VarIntDecode, VarIntEncode};
