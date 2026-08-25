//! Rotation brute force: every shift, filtered by a known-plaintext crib.

mod codec;
mod operation;

pub use operation::{Rot13BruteForce, Rot47BruteForce};
