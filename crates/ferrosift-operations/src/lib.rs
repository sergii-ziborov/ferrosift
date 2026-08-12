//! Portable pure-Rust operations for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod alphabet;
mod args;
mod base32;
mod base45;
mod base58;
mod base64;
mod base85;
mod binary;
mod bytes;
mod decimal;
mod delim;
mod escape;
mod failure;
mod find;
mod gunzip;
mod head;
mod hex;
mod hexdump;
mod identity;
mod jsint;
mod key;
mod octal;
mod spec;
mod url;
mod xor;

pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use base58::{FromBase58, ToBase58};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use binary::{FromBinary, ToBinary};
pub use bytes::{DropBytes, TakeBytes};
pub use decimal::{FromDecimal, ToDecimal};
use ferrosift_core::{OperationRegistry, RegistryError};
pub use find::FindReplace;
pub use gunzip::Gunzip;
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use hexdump::{FromHexdump, ToHexdump};
pub use identity::Identity;
pub use octal::{FromOctal, ToOctal};
pub use url::{UrlDecode, UrlEncode};
pub use xor::Xor;

/// Creates a validated registry containing all built-in operations.
///
/// # Errors
///
/// Returns [`RegistryError`] if an internal operation contract or alias is not
/// valid. The returned registry is never partially initialized.
pub fn default_registry() -> Result<OperationRegistry, RegistryError> {
    let mut registry = OperationRegistry::new();
    registry.register(Identity::new())?;
    registry.register(FromBase32::new())?;
    registry.register(ToBase32::new())?;
    registry.register(FromBase45::new())?;
    registry.register(ToBase45::new())?;
    registry.register(FromBase58::new())?;
    registry.register(ToBase58::new())?;
    registry.register(FromBase64::new())?;
    registry.register(ToBase64::new())?;
    registry.register(FromBase85::new())?;
    registry.register(ToBase85::new())?;
    registry.register(FromBinary::new())?;
    registry.register(ToBinary::new())?;
    registry.register(Gunzip::new())?;
    registry.register(DropBytes::new())?;
    registry.register(Head::new())?;
    registry.register(TakeBytes::new())?;
    registry.register(FromDecimal::new())?;
    registry.register(ToDecimal::new())?;
    registry.register(FromHex::new())?;
    registry.register(ToHex::new())?;
    registry.register(FromHexdump::new())?;
    registry.register(ToHexdump::new())?;
    registry.register(Xor::new())?;
    registry.register(FromOctal::new())?;
    registry.register(ToOctal::new())?;
    registry.register(FindReplace::new())?;
    registry.register(UrlDecode::new())?;
    registry.register(UrlEncode::new())?;
    Ok(registry)
}
