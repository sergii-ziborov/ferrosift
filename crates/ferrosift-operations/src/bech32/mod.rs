//! Bech32 (BIP-173) and Bech32m (BIP-350).

mod codec;
mod operation;

pub use operation::{FromBech32, ToBech32};
