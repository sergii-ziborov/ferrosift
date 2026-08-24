//! Which operations a build contains.
//!
//! Registration is grouped by what a group costs rather than by what it does:
//! the first four functions carry no external dependency and are always
//! present, and `register_packs` is the only one gated behind a feature.

use ferrosift_core::{OperationRegistry, RegistryError};

use crate::{
    AddLineNumbers, BitShift, Bitwise, ClassicalCipher, DropBytes, DropNthBytes, Fork, FromBase32,
    FromBase45, FromBase58, FromBase64, FromBase85, FromBinary, FromCharcode, FromDecimal, FromHex,
    FromHexdump, FromHtmlEntity, FromOctal, Head, Identity, Merge, PadLines, RemoveLineNumbers,
    RemoveNullBytes, RemoveWhitespace, Reverse, Ror13, Rot13, Rot47, Rotate, SwapEndianness, Tail,
    TakeBytes, TakeNthBytes, ToBase32, ToBase45, ToBase58, ToBase64, ToBase85, ToBinary,
    ToCharcode, ToDecimal, ToHex, ToHexdump, ToHtmlEntity, ToOctal, UrlDecode, UrlEncode, Xor,
};

#[cfg(feature = "crypto")]
use crate::{AesDecrypt, AesEncrypt, AesKeyUnwrap, AesKeyWrap, DerivePbkdf2Key, Rc4, Scrypt};
#[cfg(feature = "compression")]
use crate::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
#[cfg(feature = "text")]
use crate::{
    DefangIpAddresses, DefangUrl, ExtractDomains, ExtractEmailAddresses, ExtractFilePaths,
    ExtractHashes, ExtractIpAddresses, ExtractMacAddresses, ExtractUrls, FangUrl, FindReplace,
    Strings,
};
#[cfg(feature = "hash")]
use crate::{Hmac, Md5, Sha1, Sha2, Sha3};
#[cfg(feature = "analysis")]
use crate::{SuggestRecipe, XorBruteForce};

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
    register_text(&mut registry)?;
    register_classical(&mut registry)?;
    register_encoding(&mut registry)?;
    register_packs(&mut registry)?;
    Ok(registry)
}

/// Classical ciphers, all dependency-free.
fn register_classical(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(ClassicalCipher::a1z26_decode())?;
    registry.register(ClassicalCipher::a1z26_encode())?;
    registry.register(ClassicalCipher::affine_decode())?;
    registry.register(ClassicalCipher::affine_encode())?;
    registry.register(ClassicalCipher::atbash())?;
    registry.register(ClassicalCipher::caesar_box())?;
    registry.register(ClassicalCipher::cetacean_decode())?;
    registry.register(ClassicalCipher::cetacean_encode())?;
    registry.register(ClassicalCipher::leet())?;
    registry.register(ClassicalCipher::nato())?;
    registry.register(ClassicalCipher::rail_fence_decode())?;
    registry.register(ClassicalCipher::rail_fence_encode())?;
    registry.register(ClassicalCipher::rot8000())?;
    registry.register(ClassicalCipher::vigenere_decode())?;
    registry.register(ClassicalCipher::vigenere_encode())?;
    registry.register(Rot47::new())?;
    Ok(())
}

/// Operations that carry no external dependency and no pack gate.
fn register_core(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Identity::new())?;
    registry.register(Fork::new())?;
    registry.register(Merge::new())?;
    registry.register(DropBytes::new())?;
    registry.register(DropNthBytes::new())?;
    registry.register(Head::new())?;
    registry.register(RemoveNullBytes::new())?;
    registry.register(Reverse::new())?;
    registry.register(Ror13::new())?;
    registry.register(SwapEndianness::new())?;
    registry.register(TakeBytes::new())?;
    registry.register(TakeNthBytes::new())?;
    registry.register(Tail::new())?;
    registry.register(Xor::new())?;
    register_bitwise(registry)?;
    Ok(())
}

/// Bit-level logic, arithmetic, shifts, and rotations.
fn register_bitwise(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Bitwise::add())?;
    registry.register(Bitwise::and())?;
    registry.register(Bitwise::not())?;
    registry.register(Bitwise::or())?;
    registry.register(Bitwise::sub())?;
    registry.register(BitShift::left())?;
    registry.register(BitShift::right())?;
    registry.register(Rotate::left())?;
    registry.register(Rotate::right())?;
    Ok(())
}

/// Dependency-free text shaping: line numbering, padding, and whitespace.
fn register_text(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(AddLineNumbers::new())?;
    registry.register(PadLines::new())?;
    registry.register(RemoveLineNumbers::new())?;
    registry.register(RemoveWhitespace::new())?;
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
///
/// With no pack selected there is nothing to register, so the registry is
/// untouched; every arm below is compiled in only with its feature.
#[cfg_attr(
    not(any(
        feature = "analysis",
        feature = "compression",
        feature = "crypto",
        feature = "hash",
        feature = "text"
    )),
    expect(
        unused_variables,
        reason = "no pack is enabled, so nothing is registered"
    )
)]
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
