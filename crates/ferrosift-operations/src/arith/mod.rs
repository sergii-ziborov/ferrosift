//! Arithmetic over a delimited list of decimals.
//!
//! These are the operations the reference builds on `bignumber.js` rather than
//! on JavaScript's own numbers, and the difference is the reason they exist: a
//! float loses digits above 2^53 and rounds a tenth to something that is not a
//! tenth, while these add `0.1` and `0.2` and answer exactly `0.3`.
//!
//! Every one of them reads its input the same way, through
//! [`codec::read_list`], which keeps the tokens that read as numbers and drops
//! the rest without comment. What they do afterwards differs in one respect
//! worth naming: a total, a difference and a product are *exact*, while a
//! quotient, a mean and a standard deviation round at the twentieth place.
//! The rules for that rounding live in [`crate::jscompat::bignumber`] and are
//! pinned against the reference library directly.
//!
//! MOD is the odd one. The others hand back a number and let the dish render
//! it with `toFixed`; MOD joins its results into a string itself, which in the
//! reference means `toString` -- and `toString` uses exponential notation
//! where `toFixed` never does.

mod codec;
mod operation;

pub use operation::{Aggregate, Mod};
