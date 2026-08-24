//! Classical ciphers: substitution, transposition, and spelling alphabets.
//!
//! All of them are dependency-free and operate on JavaScript strings, so the
//! ports work in UTF-16 code units to match the reference's indexing.

mod a1z26;
mod affine;
mod catalog;
mod letters;
mod operation;
mod rot;
mod transpose;
mod whimsy;

pub use operation::{ClassicalCipher, Rot47};
