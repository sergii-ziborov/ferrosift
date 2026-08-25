//! Consistent Overhead Byte Stuffing.
//!
//! COBS removes every zero byte from a payload at a cost of roughly one byte
//! per 254, which lets a zero serve as an unambiguous frame delimiter. That is
//! why it turns up in serial protocols, and why the encoder's output is worth
//! pinning byte-for-byte rather than treating any zero-free encoding as
//! equivalent: a receiver splits on the delimiter the sender chose.
//!
//! The reference's decoder tolerates a truncated final block, returning the
//! bytes it did receive rather than reporting the shortfall. That is reproduced
//! rather than tightened — a stricter port would refuse frames the reference
//! accepts, which is the same kind of silent disagreement as accepting frames
//! it refuses.

mod codec;
mod operation;

pub use operation::{FromCobs, ToCobs};
