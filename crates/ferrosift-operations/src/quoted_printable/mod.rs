//! Quoted-printable, the encoding that keeps text readable.
//!
//! Its job is to survive a mail path that may only carry seven-bit data, while
//! leaving anything already printable alone — so an English message stays
//! legible and only the accented characters turn into `=XX`.
//!
//! Two details carry most of the behaviour. Trailing spaces and tabs are
//! escaped, because a mail transfer agent is free to strip them and the
//! message would otherwise change in transit. And lines are broken at 76
//! characters with a trailing `=`, chosen so as not to split an escape in half
//! nor to split a multi-byte UTF-8 character written as consecutive escapes —
//! neither of which the specification demands, and both of which the reference
//! does.
//!
//! The decoder previously lived beside `VarInt` for no reason other than
//! arrival order. It is here now, next to its inverse.

mod codec;
mod operation;

pub use operation::{FromQuotedPrintable, ToQuotedPrintable};
