//! Windows filetimes and UNIX timestamps.
//!
//! Two conversions between epochs, and the reason they wait on an
//! arbitrary-precision layer rather than on a date library: a filetime counts
//! hundred-nanosecond intervals since 1601, so an ordinary timestamp lands
//! around `1.3 x 10^17` and a nanosecond one goes further. That is past what a
//! double counts by ones, and a port on floats would be quietly wrong in the
//! last digits of every answer.
//!
//! No calendar arithmetic happens here. The whole conversion is a
//! multiplication, an addition of one constant, and a rendering -- which is
//! why these are the reference's own operations rather than a date library's.

mod codec;
mod operation;

pub use operation::{FiletimeToUnix, UnixToFiletime};
