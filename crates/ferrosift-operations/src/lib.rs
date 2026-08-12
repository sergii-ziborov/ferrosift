//! Portable pure-Rust operations for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod aes_kw;
mod aes_op;
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
mod codec_bytes;
mod compress;
mod crc32;
mod decimal;
mod defang;
mod delim;
mod escape;
mod extract;
mod failure;
mod find;
mod hash;
mod head;
mod hex;
mod hex_util;
mod hexdump;
mod hmac_op;
mod html;
mod identity;
mod jsint;
mod kdf;
mod key;
mod octal;
mod rc4_op;
mod rot13;
mod spec;
mod suggest;
mod url;
mod xor;
mod xor_brute;

pub use aes_kw::{AesKeyUnwrap, AesKeyWrap};
pub use aes_op::{AesDecrypt, AesEncrypt};
pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use base58::{FromBase58, ToBase58};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use binary::{FromBinary, ToBinary};
pub use bytes::{DropBytes, TakeBytes};
pub use charcode::{FromCharcode, ToCharcode};
pub use compress::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
pub use decimal::{FromDecimal, ToDecimal};
pub use defang::{DefangIpAddresses, DefangUrl, FangUrl};
pub use extract::{
    ExtractDomains, ExtractEmailAddresses, ExtractFilePaths, ExtractHashes, ExtractIpAddresses,
    ExtractMacAddresses, ExtractUrls, Strings,
};
use ferrosift_core::{OperationRegistry, RegistryError};
pub use find::FindReplace;
pub use hash::{Md5, Sha1, Sha2, Sha3};
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use hexdump::{FromHexdump, ToHexdump};
pub use hmac_op::Hmac;
pub use html::{FromHtmlEntity, ToHtmlEntity};
pub use identity::Identity;
pub use kdf::{DerivePbkdf2Key, Scrypt};
pub use octal::{FromOctal, ToOctal};
pub use rc4_op::Rc4;
pub use rot13::Rot13;
pub use suggest::SuggestRecipe;
pub use url::{UrlDecode, UrlEncode};
pub use xor::Xor;
pub use xor_brute::XorBruteForce;

/// Creates a validated registry containing all built-in operations.
///
/// # Errors
///
/// Returns [`RegistryError`] if an internal operation contract or alias is not
/// valid. The returned registry is never partially initialized.
pub fn default_registry() -> Result<OperationRegistry, RegistryError> {
    let mut registry = OperationRegistry::new();
    registry.register(Identity::new())?;
    registry.register(SuggestRecipe::new())?;
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
    registry.register(Bzip2Compress::new())?;
    registry.register(Bzip2Decompress::new())?;
    registry.register(Gunzip::new())?;
    registry.register(Gzip::new())?;
    registry.register(RawDeflate::new())?;
    registry.register(RawInflate::new())?;
    registry.register(ZlibDeflate::new())?;
    registry.register(ZlibInflate::new())?;
    registry.register(DropBytes::new())?;
    registry.register(Head::new())?;
    registry.register(TakeBytes::new())?;
    registry.register(DefangIpAddresses::new())?;
    registry.register(DefangUrl::new())?;
    registry.register(FangUrl::new())?;
    registry.register(FromDecimal::new())?;
    registry.register(ToDecimal::new())?;
    registry.register(ExtractDomains::new())?;
    registry.register(ExtractEmailAddresses::new())?;
    registry.register(ExtractFilePaths::new())?;
    registry.register(ExtractHashes::new())?;
    registry.register(ExtractIpAddresses::new())?;
    registry.register(ExtractMacAddresses::new())?;
    registry.register(ExtractUrls::new())?;
    registry.register(Strings::new())?;
    registry.register(Md5::new())?;
    registry.register(Sha1::new())?;
    registry.register(Sha2::new())?;
    registry.register(Sha3::new())?;
    registry.register(Hmac::new())?;
    registry.register(FromHex::new())?;
    registry.register(ToHex::new())?;
    registry.register(FromHexdump::new())?;
    registry.register(ToHexdump::new())?;
    registry.register(FromHtmlEntity::new())?;
    registry.register(ToHtmlEntity::new())?;
    registry.register(Xor::new())?;
    registry.register(XorBruteForce::new())?;
    registry.register(AesDecrypt::new())?;
    registry.register(AesEncrypt::new())?;
    registry.register(AesKeyUnwrap::new())?;
    registry.register(AesKeyWrap::new())?;
    registry.register(DerivePbkdf2Key::new())?;
    registry.register(Rc4::new())?;
    registry.register(Scrypt::new())?;
    registry.register(FromOctal::new())?;
    registry.register(ToOctal::new())?;
    registry.register(Rot13::new())?;
    registry.register(FindReplace::new())?;
    registry.register(UrlDecode::new())?;
    registry.register(UrlEncode::new())?;
    Ok(registry)
}
