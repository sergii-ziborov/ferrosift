//! Which operations a build contains.
//!
//! Registration is grouped by what a group costs rather than by what it does:
//! the first four functions carry no external dependency and are always
//! present, and `register_packs` is the only one gated behind a feature.

use ferrosift_core::{OperationRegistry, RegistryError};

use crate::{
    AddLineNumbers, AlternatingCaps, BitShift, Bitwise, CaretMDecode, Checksum, ClassicalCipher,
    DechunkHttpResponse, DecodeNetbiosName, DropBytes, DropNthBytes, EncodeNetbiosName,
    EscapeSmartCharacters, EscapeUnicodeCharacters, ExpandAlphabetRange, Fork, FormatMacAddresses,
    FromBraille, FromCaseInsensitiveRegex, FromQuotedPrintable, GenerateDeBruijnSequence,
    GetAllCasings, HtmlToText, ParityBit, PowerSet, RemoveAnsiEscapeCodes, StripHtmlTags,
    StripHttpHeaders, Substitute, SwapCase, ToBraille, ToLowerCase, ToUpperCase, UnescapeString,
    UnescapeUnicodeCharacters, UnicodeTextFormat, VarIntDecode, VarIntEncode, Wrap,
};
use crate::{
    CitrixCtx1Decode, CitrixCtx1Encode, FromBase32, FromBase45, FromBase58, FromBase64, FromBase85,
    FromBinary, FromCharcode, FromCobs, FromDecimal, FromFloat, FromHex, FromHexdump,
    FromHtmlEntity, FromModhex, FromMorseCode, FromOctal, HammingDistance, Head, HexToPem,
    Identity, LevenshteinDistance, LuhnChecksum, Merge, PadLines, PemToHex, RemoveLineNumbers,
    RemoveNullBytes, RemoveWhitespace, Reverse, Ror13, Rot13, Rot13BruteForce, Rot47,
    Rot47BruteForce, Rotate, SetOperation, Split, SwapEndianness, Tail, TakeBytes, TakeNthBytes,
    ToBase32, ToBase45, ToBase58, ToBase64, ToBase85, ToBinary, ToCharcode, ToCobs, ToDecimal,
    ToFloat, ToHex, ToHexdump, ToHtmlEntity, ToModhex, ToMorseCode, ToOctal, ToQuotedPrintable,
    Unique, UrlDecode, UrlEncode, Xor,
};

#[cfg(feature = "crypto")]
use crate::{AesDecrypt, AesEncrypt, AesKeyUnwrap, AesKeyWrap, DerivePbkdf2Key, Rc4, Scrypt};
#[cfg(feature = "compression")]
use crate::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
#[cfg(feature = "text")]
use crate::{
    CountOccurrences, DefangIpAddresses, DefangUrl, ExtractDomains, ExtractEmailAddresses,
    ExtractFilePaths, ExtractHashes, ExtractIpAddresses, ExtractMacAddresses, ExtractUrls, FangUrl,
    FindReplace, Strings,
};
#[cfg(feature = "arithmetic")]
use crate::{ExtendedGcd, ModularInverse};
#[cfg(feature = "hash")]
use crate::{FixedDigest, Hmac, Md5, NtHash, Ripemd, Sha1, Sha2, Sha3};
#[cfg(feature = "bignum")]
use crate::{FromBase62, HexToObjectIdentifier, ObjectIdentifierToHex, ToBase62};
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
    register_checksums(&mut registry)?;
    register_sets(&mut registry)?;
    register_core(&mut registry)?;
    register_text(&mut registry)?;
    register_classical(&mut registry)?;
    register_encoding(&mut registry)?;
    register_packs(&mut registry)?;
    Ok(registry)
}

/// Set operations and edit distances, all dependency-free.
fn register_sets(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(SetOperation::cartesian_product())?;
    registry.register(SetOperation::difference())?;
    registry.register(SetOperation::intersection())?;
    registry.register(SetOperation::symmetric_difference())?;
    registry.register(SetOperation::union())?;
    registry.register(HammingDistance::new())?;
    registry.register(LevenshteinDistance::new())?;
    Ok(())
}

/// Checksums, all dependency-free.
fn register_checksums(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Checksum::adler32())?;
    registry.register(Checksum::fletcher8())?;
    registry.register(Checksum::fletcher16())?;
    registry.register(Checksum::fletcher32())?;
    registry.register(Checksum::fletcher64())?;
    registry.register(Checksum::tcp_ip())?;
    registry.register(Checksum::xor())?;
    registry.register(LuhnChecksum::new())?;
    Ok(())
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
    registry.register(Rot13BruteForce::new())?;
    registry.register(Rot47BruteForce::new())?;
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
    register_casing(registry)?;
    register_shape(registry)?;
    Ok(())
}

/// Case transforms.
///
/// These carry no feature gate: they are string-to-string transforms with no
/// tables and no dependencies, so a build that has text values at all can
/// afford them.
fn register_casing(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(ToLowerCase::new())?;
    registry.register(ToUpperCase::new())?;
    registry.register(SwapCase::new())?;
    registry.register(AlternatingCaps::new())?;
    registry.register(GetAllCasings::new())?;
    Ok(())
}

/// Reshaping text: ANSI stripping, HTTP framing, wrapping, and ranges.
///
/// Ungated for the same reason as the case transforms: no tables, no
/// dependencies. The two HTTP operations parse framing rather than speak the
/// protocol, so they need no host handle either.
fn register_shape(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Unique::new())?;
    registry.register(Split::new())?;
    registry.register(RemoveAnsiEscapeCodes::new())?;
    registry.register(StripHttpHeaders::new())?;
    registry.register(DechunkHttpResponse::new())?;
    registry.register(Wrap::new())?;
    registry.register(ExpandAlphabetRange::new())?;
    registry.register(CaretMDecode::new())?;
    registry.register(FromCaseInsensitiveRegex::new())?;
    registry.register(PowerSet::new())?;
    registry.register(Substitute::new())?;
    registry.register(UnescapeString::new())?;
    registry.register(GenerateDeBruijnSequence::new())?;
    registry.register(ParityBit::new())?;
    registry.register(FormatMacAddresses::new())?;
    registry.register(EscapeSmartCharacters::new())?;
    registry.register(StripHtmlTags::new())?;
    registry.register(VarIntEncode::new())?;
    registry.register(VarIntDecode::new())?;
    registry.register(FromQuotedPrintable::new())?;
    registry.register(ToQuotedPrintable::new())?;
    registry.register(HexToPem::new())?;
    registry.register(PemToHex::new())?;
    registry.register(ToBraille::new())?;
    registry.register(FromBraille::new())?;
    registry.register(UnicodeTextFormat::new())?;
    registry.register(HtmlToText::new())?;
    registry.register(EscapeUnicodeCharacters::new())?;
    registry.register(UnescapeUnicodeCharacters::new())?;
    registry.register(EncodeNetbiosName::new())?;
    registry.register(DecodeNetbiosName::new())?;
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
///
/// Except Base62, which is registered with the `bignum` pack instead: 62 is
/// not a power of two, so the whole input is one integer rather than a stream
/// of bit groups.
fn register_encoding(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(FromBase32::new())?;
    registry.register(ToBase32::new())?;
    registry.register(FromBase45::new())?;
    registry.register(ToBase45::new())?;
    registry.register(FromBase58::new())?;
    registry.register(ToBase58::new())?;
    registry.register(CitrixCtx1Decode::new())?;
    registry.register(CitrixCtx1Encode::new())?;
    registry.register(FromCobs::new())?;
    registry.register(FromFloat::new())?;
    registry.register(ToFloat::new())?;
    registry.register(ToCobs::new())?;
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
    registry.register(FromModhex::new())?;
    registry.register(ToModhex::new())?;
    registry.register(FromMorseCode::new())?;
    registry.register(ToMorseCode::new())?;
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
        feature = "arithmetic",
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
    #[cfg(feature = "arithmetic")]
    {
        registry.register(ExtendedGcd::new())?;
        registry.register(ModularInverse::new())?;
    }
    // Encodings rather than arithmetic, but they need the same big integers,
    // so they follow the dependency and not the subject.
    #[cfg(feature = "bignum")]
    {
        registry.register(FromBase62::new())?;
        registry.register(ToBase62::new())?;
        registry.register(HexToObjectIdentifier::new())?;
        registry.register(ObjectIdentifierToHex::new())?;
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
        registry.register(CountOccurrences::new())?;
    }
    #[cfg(feature = "hash")]
    {
        registry.register(Md5::new())?;
        registry.register(Sha1::new())?;
        registry.register(Sha2::new())?;
        registry.register(Sha3::new())?;
        registry.register(FixedDigest::md2())?;
        registry.register(FixedDigest::md4())?;
        registry.register(FixedDigest::sm3())?;
        registry.register(FixedDigest::whirlpool())?;
        registry.register(NtHash::new())?;
        registry.register(Ripemd::new())?;
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
