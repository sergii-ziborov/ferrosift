//! Reading and writing a number in another base.
//!
//! Not a byte encoding, despite sitting beside them: `To Base64` rewrites
//! bytes, while these two rewrite a *number*. The input to one and the output
//! of the other is a decimal of unbounded size, which is why they wait on the
//! same arbitrary-precision layer the arithmetic operations do.
//!
//! The two are not quite inverses of each other, and the reason is worth
//! knowing before reading either. `To Base` hands its value straight to the
//! reference's `toString(base)`. `From Base` does *not* hand its text to the
//! matching constructor: it splits on the point itself and reads each
//! fractional digit alone. So a value whose letters mix case is read by one
//! and refused by the other, and a fraction rounds once per digit here where
//! the constructor would round once for the whole.

mod codec;
mod operation;

pub use operation::{FromBase, ToBase};
