//! Bacon's cipher: five binary symbols per letter, written four different ways.

mod codec;
mod operation;

pub use operation::{BaconDecode, BaconEncode};
