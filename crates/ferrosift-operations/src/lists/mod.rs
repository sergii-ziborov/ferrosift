//! Reshaping a delimited list.
//!
//! Two operations that look trivial and are not, both for the same reason:
//! JavaScript's `split` and JavaScript's objects behave in ways a direct
//! reading would miss.
//!
//! `Unique` with counts does not report entries in first-seen order. The
//! reference tallies into a plain object and then reads `Object.keys`, which
//! hands back integer-like keys first in ascending numeric order and only then
//! the rest in insertion order — so counting `b, 2, a, 1` lists `1` and `2`
//! first, having reordered them on the way. That ordering lives in
//! [`crate::jscompat::object`] because several operations inherit it.
//!
//! `Split`'s two delimiters are literal text rather than delimiter names, so
//! its default join delimiter is a backslash followed by `n` and not a line
//! feed. That is the reference's default and it is preserved rather than
//! corrected: a recipe that relies on it would otherwise change meaning.

mod codec;
mod operation;

pub use operation::{Split, Unique};
