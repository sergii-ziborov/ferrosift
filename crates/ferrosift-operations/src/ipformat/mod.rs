//! One IPv4 address, written four different ways.
//!
//! Dotted decimal, a single decimal, a single octal with a leading zero, and
//! eight hexadecimal digits. The conversion goes through four bytes in every
//! case, so a value that does not fit in four bytes is not rejected -- it is
//! truncated to the low thirty-two bits, because the reference builds the
//! octets with shifts and masks and JavaScript's shifts work on thirty-two.
//!
//! Every line is converted on its own and empty lines are dropped, which makes
//! the operation a list transformation rather than a single conversion.

mod codec;
mod operation;

pub use operation::ChangeIpFormat;
