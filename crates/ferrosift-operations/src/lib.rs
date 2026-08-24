//! Portable pure-Rust operations for `FerroSift`.
//!
//! The catalog is split into feature packs so a deployment compiles only what
//! it uses. Identity, flow control, every encoding, byte slicing,
//! and XOR carry no external dependency and are always present;
//! `hash`, `crypto`, `compression`, `text`, and `analysis` are opt-in and are
//! the only packs that pull third-party crates. `portable-full` selects them
//! all and is the default.

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
mod charcode;
mod decimal;
mod delim;
mod escape;
mod failure;
mod flow;
mod head;
mod hex;
mod hex_util;
mod hexdump;
mod html;
mod identity;
mod jsint;
mod key;
mod octal;
mod rot13;
mod spec;
mod url;
mod xor;

#[cfg(feature = "crypto")]
mod aes_kw;
#[cfg(feature = "crypto")]
mod aes_op;
#[cfg(feature = "crypto")]
mod codec_bytes;
#[cfg(feature = "compression")]
mod compress;
#[cfg(feature = "compression")]
mod crc32;
#[cfg(feature = "text")]
mod defang;
#[cfg(feature = "text")]
mod extract;
#[cfg(feature = "text")]
mod find;
#[cfg(feature = "hash")]
mod hash;
#[cfg(feature = "hash")]
mod hmac_op;
#[cfg(feature = "crypto")]
mod kdf;
#[cfg(feature = "crypto")]
mod rc4_op;
#[cfg(feature = "analysis")]
mod suggest;
#[cfg(feature = "analysis")]
mod xor_brute;

pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use base58::{FromBase58, ToBase58};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use binary::{FromBinary, ToBinary};
pub use bytes::{DropBytes, TakeBytes};
pub use charcode::{FromCharcode, ToCharcode};
pub use decimal::{FromDecimal, ToDecimal};
use ferrosift_core::{OperationRegistry, RegistryError};
pub use flow::{Fork, Merge};
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use hexdump::{FromHexdump, ToHexdump};
pub use html::{FromHtmlEntity, ToHtmlEntity};
pub use identity::Identity;
pub use octal::{FromOctal, ToOctal};
pub use rot13::Rot13;
pub use url::{UrlDecode, UrlEncode};
pub use xor::Xor;

#[cfg(feature = "crypto")]
pub use aes_kw::{AesKeyUnwrap, AesKeyWrap};
#[cfg(feature = "crypto")]
pub use aes_op::{AesDecrypt, AesEncrypt};
#[cfg(feature = "compression")]
pub use compress::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
#[cfg(feature = "text")]
pub use defang::{DefangIpAddresses, DefangUrl, FangUrl};
#[cfg(feature = "text")]
pub use extract::{
    ExtractDomains, ExtractEmailAddresses, ExtractFilePaths, ExtractHashes, ExtractIpAddresses,
    ExtractMacAddresses, ExtractUrls, Strings,
};
#[cfg(feature = "text")]
pub use find::FindReplace;
#[cfg(feature = "hash")]
pub use hash::{Md5, Sha1, Sha2, Sha3};
#[cfg(feature = "hash")]
pub use hmac_op::Hmac;
#[cfg(feature = "crypto")]
pub use kdf::{DerivePbkdf2Key, Scrypt};
#[cfg(feature = "crypto")]
pub use rc4_op::Rc4;
#[cfg(feature = "analysis")]
pub use suggest::SuggestRecipe;
#[cfg(feature = "analysis")]
pub use xor_brute::XorBruteForce;

/// Creates a validated registry containing every enabled built-in operation.
///
/// Which operations are present depends on the selected feature packs; with
/// the default `portable-full` this is the whole portable catalog.
///
/// # Errors
///
/// Returns [`RegistryError`] if an internal operation contract or alias is not
/// valid. The returned registry is never partially initialized.
pub fn default_registry() -> Result<OperationRegistry, RegistryError> {
    let mut registry = OperationRegistry::new();
    register_core(&mut registry)?;
    register_encoding(&mut registry)?;
    register_packs(&mut registry)?;
    Ok(registry)
}

/// Operations that carry no external dependency and no pack gate.
fn register_core(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Identity::new())?;
    registry.register(Fork::new())?;
    registry.register(Merge::new())?;
    registry.register(DropBytes::new())?;
    registry.register(Head::new())?;
    registry.register(TakeBytes::new())?;
    registry.register(Xor::new())?;
    Ok(())
}

/// Every representation codec, all dependency-free.
fn register_encoding(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
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
    registry.register(FromCharcode::new())?;
    registry.register(ToCharcode::new())?;
    registry.register(FromDecimal::new())?;
    registry.register(ToDecimal::new())?;
    registry.register(FromHex::new())?;
    registry.register(ToHex::new())?;
    registry.register(FromHexdump::new())?;
    registry.register(ToHexdump::new())?;
    registry.register(FromHtmlEntity::new())?;
    registry.register(ToHtmlEntity::new())?;
    registry.register(FromOctal::new())?;
    registry.register(ToOctal::new())?;
    registry.register(Rot13::new())?;
    registry.register(UrlDecode::new())?;
    registry.register(UrlEncode::new())?;
    Ok(())
}

/// Operations gated behind the opt-in packs.
fn register_packs(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    #[cfg(feature = "analysis")]
    {
        registry.register(SuggestRecipe::new())?;
        registry.register(XorBruteForce::new())?;
    }
    #[cfg(feature = "compression")]
    {
        registry.register(Bzip2Compress::new())?;
        registry.register(Bzip2Decompress::new())?;
        registry.register(Gunzip::new())?;
        registry.register(Gzip::new())?;
        registry.register(RawDeflate::new())?;
        registry.register(RawInflate::new())?;
        registry.register(ZlibDeflate::new())?;
        registry.register(ZlibInflate::new())?;
    }
    #[cfg(feature = "text")]
    {
        registry.register(FindReplace::new())?;
        registry.register(DefangIpAddresses::new())?;
        registry.register(DefangUrl::new())?;
        registry.register(FangUrl::new())?;
        registry.register(ExtractDomains::new())?;
        registry.register(ExtractEmailAddresses::new())?;
        registry.register(ExtractFilePaths::new())?;
        registry.register(ExtractHashes::new())?;
        registry.register(ExtractIpAddresses::new())?;
        registry.register(ExtractMacAddresses::new())?;
        registry.register(ExtractUrls::new())?;
        registry.register(Strings::new())?;
    }
    #[cfg(feature = "hash")]
    {
        registry.register(Md5::new())?;
        registry.register(Sha1::new())?;
        registry.register(Sha2::new())?;
        registry.register(Sha3::new())?;
        registry.register(Hmac::new())?;
    }
    #[cfg(feature = "crypto")]
    {
        registry.register(AesDecrypt::new())?;
        registry.register(AesEncrypt::new())?;
        registry.register(AesKeyUnwrap::new())?;
        registry.register(AesKeyWrap::new())?;
        registry.register(DerivePbkdf2Key::new())?;
        registry.register(Rc4::new())?;
        registry.register(Scrypt::new())?;
    }
    Ok(())
}
