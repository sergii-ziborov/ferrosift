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
mod decimal;
mod delim;
mod failure;
mod hex;
mod identity;
mod jsint;
mod octal;
mod spec;
mod url;

pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use base58::{FromBase58, ToBase58};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use binary::{FromBinary, ToBinary};
pub use decimal::{FromDecimal, ToDecimal};
use ferrosift_core::{OperationRegistry, RegistryError};
pub use hex::{FromHex, ToHex};
pub use identity::Identity;
pub use octal::{FromOctal, ToOctal};
pub use url::{UrlDecode, UrlEncode};

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
    registry.register(FromDecimal::new())?;
    registry.register(ToDecimal::new())?;
    registry.register(FromHex::new())?;
    registry.register(ToHex::new())?;
    registry.register(FromOctal::new())?;
    registry.register(ToOctal::new())?;
    registry.register(UrlDecode::new())?;
    registry.register(UrlEncode::new())?;
    Ok(registry)
}
