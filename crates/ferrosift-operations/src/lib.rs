//! Portable pure-Rust operations for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod alphabet;
mod args;
mod base64;
mod failure;
mod hex;
mod identity;
mod spec;

pub use base64::{FromBase64, ToBase64};
use ferrosift_core::{OperationRegistry, RegistryError};
pub use hex::{FromHex, ToHex};
pub use identity::Identity;

/// Creates a validated registry containing all built-in operations.
///
/// # Errors
///
/// Returns [`RegistryError`] if an internal operation contract or alias is not
/// valid. The returned registry is never partially initialized.
pub fn default_registry() -> Result<OperationRegistry, RegistryError> {
    let mut registry = OperationRegistry::new();
    registry.register(Identity::new())?;
    registry.register(FromBase64::new())?;
    registry.register(ToBase64::new())?;
    registry.register(FromHex::new())?;
    registry.register(ToHex::new())?;
    Ok(registry)
}
