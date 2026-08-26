//! Which operations a build contains.
//!
//! Registration is grouped by *family* — what an operation is for — rather
//! than by what it costs to compile. That is a deliberate reversal. Grouping
//! by cost put every hash in one function because they share a feature gate,
//! and every encoding in another because they share the absence of one, so
//! "where does a new hash go" and "where does a new encoding go" had different
//! answers for a reason that had nothing to do with either. Feature gates now
//! sit inside the family they belong to.
//!
//! The families are a table rather than a sequence of calls, and each one
//! declares which catalog categories it accepts. `tests/registry.rs` builds
//! each family on its own and fails if an operation landed somewhere its own
//! specification does not agree with. That is what stops a family from
//! becoming the junk drawer the old `register_shape` had turned into — it held
//! HTTP framing, Braille, `NetBIOS` names, and `PowerSet`, none of which is
//! shaping anything.
//!
//! Where a category is too small to be its own family it is merged into a
//! neighbour, and the merge is named in the table rather than left implicit.

use ferrosift_core::{OperationRegistry, RegistryError};

use crate::{
    AddLineNumbers, AlternatingCaps, BaconDecode, BaconEncode, BifidCipher, BitShift, Bitwise,
    CaretMDecode, Checksum, ChiSquare, CitrixCtx1Decode,
    CitrixCtx1Encode, ClassicalCipher, Comment, DechunkHttpResponse, DecodeNetbiosName, DropBytes,
    DropNthBytes, EncodeNetbiosName, EscapeSmartCharacters, EscapeUnicodeCharacters,
    ExpandAlphabetRange, Fork, FormatMacAddresses, FromBase32, FromBase45, FromBase58, FromBase64,
    FromBase85, FromBech32, FromBinary, FromBraille, FromCaseInsensitiveRegex, FromCharcode,
    FromCobs,
    FromBase92, FromDecimal, FromFloat, FromHex, FromHexContent, FromHexdump, FromHtmlEntity,
    FromModhex, FromMorseCode,
    FromOctal, FromQuotedPrintable, GenerateDeBruijnSequence, GetAllCasings, HammingDistance, Head,
    HexToPem, HtmlToText, Identity, IndexOfCoincidence, LevenshteinDistance, Ls47Decrypt, Ls47Encrypt, LuhnChecksum, Merge, MurmurHash3,
    MicrosoftScriptDecoder, OffsetChecker, PadLines, ParityBit, ParseColourCode, ParseUnixFilePermissions, Punycode,
    PemToHex, PowerSet, RemoveAnsiEscapeCodes, RemoveLineNumbers, RemoveNullBytes,
    RemoveWhitespace, Reverse, Ror13, Rot13, Rot13BruteForce, Rot47, Rot47BruteForce, Rotate,
    SetOperation, Sha0, Split, StripHeader, StripHtmlTags, StripHttpHeaders, Substitute, SwapCase,
    SwapEndianness,
    Tail, TakeBytes, TakeNthBytes, ToBase32, ToBase45, ToBase58, ToBase64, ToBase85, ToBase92,
    ToBech32,
    ToBinary, ToBraille, ToCaseInsensitiveRegex, ToCharcode, ToCobs, ToDecimal, ToFloat, ToHex,
    ToHexContent, ToHexdump,
    ToHtmlEntity, ToLowerCase, ToTable,
    ToModhex, ToMorseCode, ToOctal, ToQuotedPrintable, ToUpperCase, UnescapeString,
    UnescapeUnicodeCharacters, UnicodeTextFormat, Unique, UrlDecode, UrlEncode, VarIntDecode,
    VarIntEncode, Wrap, XkcdRandomNumber, Xor, Xxtea,
};

#[cfg(feature = "crypto")]
use crate::{
    AesDecrypt, AesEncrypt, AesKeyUnwrap, AesKeyWrap, DerivePbkdf2Key, Rc4, Rc4Drop, Scrypt,
};
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
use crate::{FixedDigest, Hmac, Keccak, Md5, NtHash, Ripemd, Sha1, Sha2, Sha3, Shake};
#[cfg(feature = "bignum")]
use crate::{FromBase62, HexToObjectIdentifier, ObjectIdentifierToHex, ToBase62};
#[cfg(feature = "analysis")]
use crate::{SuggestRecipe, XorBruteForce};

/// One family of operations, and the catalog categories it is allowed to hold.
pub(crate) struct Family {
    /// What to call this family when a test reports a mismatch.
    pub name: &'static str,
    /// Categories whose operations may be registered here.
    ///
    /// More than one only where a category is too small to stand alone; the
    /// doc comment on each entry says why that merge is the right one.
    pub categories: &'static [&'static str],
    /// Adds this family's operations to a registry.
    pub register: fn(&mut OperationRegistry) -> Result<(), RegistryError>,
}

/// Every family, in the order `default_registry` builds them.
///
/// Order is presentation only — the registry is keyed by operation id, and
/// nothing downstream depends on insertion order.
pub(crate) const FAMILIES: &[Family] = &[
    Family {
        name: "analysis",
        categories: &["Analysis"],
        register: register_analysis,
    },
    Family {
        name: "arithmetic",
        categories: &["Arithmetic"],
        register: register_arithmetic,
    },
    Family {
        name: "checksums",
        categories: &["Checksums"],
        register: register_checksums,
    },
    Family {
        // Key derivation is here rather than alone: a KDF is what turns a
        // password into a key, and the only reason to want one is to feed a
        // cipher that is also here.
        name: "ciphers",
        categories: &["Ciphers", "KDF"],
        register: register_ciphers,
    },
    Family {
        name: "compression",
        categories: &["Compression"],
        register: register_compression,
    },
    Family {
        // Slicing and reordering raw bytes, plus Identity, which is the
        // do-nothing operation and belongs with the byte primitives rather
        // than with control flow.
        name: "data",
        categories: &["Data", "Core"],
        register: register_data,
    },
    Family {
        name: "encoding",
        categories: &["Encoding"],
        register: register_encoding,
    },
    Family {
        // Defanging is what you do to the indicators extraction finds, so the
        // two travel together and share the `text` pack.
        name: "extractors",
        categories: &["Extractors", "Defang"],
        register: register_extractors,
    },
    Family {
        name: "flow",
        categories: &["Flow control"],
        register: register_flow,
    },
    Family {
        name: "hashing",
        categories: &["Hashing"],
        register: register_hashing,
    },
    Family {
        name: "logic",
        categories: &["Logic"],
        register: register_logic,
    },
    Family {
        // Reading a structured format rather than transforming bytes: object
        // identifiers, PEM blocks, and one address format.
        name: "parsing",
        categories: &["Parsing", "Networking"],
        register: register_parsing,
    },
    Family {
        // Both are comparisons over lists — one asks what two sets share, the
        // other asks how far apart two strings are.
        name: "sets",
        categories: &["Sets", "Distance"],
        register: register_sets,
    },
    Family {
        // Splitting and deduplicating a delimited list is text handling; the
        // `Shaping` category is two operations and does not earn a family.
        name: "text",
        categories: &["Text", "Shaping"],
        register: register_text,
    },
];

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
    for family in FAMILIES {
        (family.register)(&mut registry)?;
    }
    Ok(registry)
}

/// Recipe suggestion and brute-force search.
#[cfg_attr(
    not(feature = "analysis"),
    expect(unused_variables, reason = "the analysis pack is not enabled")
)]
fn register_analysis(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    // Outside the feature gate: both are arithmetic over the input and pull
    // nothing, so gating them would remove an operation for a cost it does
    // not have.
    registry.register(ChiSquare::new())?;
    registry.register(IndexOfCoincidence::new())?;
    registry.register(OffsetChecker::new())?;

    #[cfg(feature = "analysis")]
    {
        registry.register(SuggestRecipe::new())?;
    }
    Ok(())
}

/// Arbitrary-precision integer arithmetic.
#[cfg_attr(
    not(feature = "arithmetic"),
    expect(unused_variables, reason = "the arithmetic pack is not enabled")
)]
fn register_arithmetic(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    #[cfg(feature = "arithmetic")]
    {
        registry.register(ExtendedGcd::new())?;
        registry.register(ModularInverse::new())?;
    }
    Ok(())
}

/// Checksums: cheap integrity, not integrity against an adversary.
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

/// Ciphers and key derivation, classical and modern.
///
/// The classical half needs no dependency and the modern half needs the
/// `crypto` pack, which is a difference in cost rather than in kind — so both
/// live here and the gate sits around the half that needs it.
fn register_ciphers(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(BaconDecode::new())?;
    registry.register(BaconEncode::new())?;
    registry.register(BifidCipher::decode())?;
    registry.register(BifidCipher::encode())?;
    registry.register(Ls47Encrypt::new())?;
    registry.register(Xxtea::encrypt())?;
    registry.register(Xxtea::decrypt())?;
    registry.register(Ls47Decrypt::new())?;
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
    registry.register(Substitute::new())?;

    #[cfg(feature = "crypto")]
    {
        registry.register(AesDecrypt::new())?;
        registry.register(AesEncrypt::new())?;
        registry.register(AesKeyUnwrap::new())?;
        registry.register(AesKeyWrap::new())?;
        registry.register(Rc4::new())?;
    registry.register(Rc4Drop::new())?;
        registry.register(DerivePbkdf2Key::new())?;
        registry.register(Scrypt::new())?;
    }
    Ok(())
}

/// Compressors and their inverses.
#[cfg_attr(
    not(feature = "compression"),
    expect(unused_variables, reason = "the compression pack is not enabled")
)]
fn register_compression(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
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
    Ok(())
}

/// Slicing, reordering, and passing bytes through untouched.
fn register_data(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Identity::new())?;
    registry.register(DropBytes::new())?;
    registry.register(DropNthBytes::new())?;
    registry.register(Head::new())?;
    registry.register(RemoveNullBytes::new())?;
    registry.register(Reverse::new())?;
    registry.register(SwapEndianness::new())?;
    registry.register(TakeBytes::new())?;
    registry.register(TakeNthBytes::new())?;
    Ok(())
}

/// Every representation codec.
fn register_encoding(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Punycode::encode())?;
    registry.register(Punycode::decode())?;
    registry.register(MicrosoftScriptDecoder::new())?;
    registry.register(ToBech32::new())?;
    registry.register(FromBech32::new())?;
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
    registry.register(FromBase92::new())?;
    registry.register(ToBase92::new())?;
    registry.register(FromHexContent::new())?;
    registry.register(ToHexContent::new())?;
    registry.register(FromBinary::new())?;
    registry.register(ToBinary::new())?;
    registry.register(FromBraille::new())?;
    registry.register(ToBraille::new())?;
    registry.register(FromCharcode::new())?;
    registry.register(ToCharcode::new())?;
    registry.register(CaretMDecode::new())?;
    registry.register(FromCobs::new())?;
    registry.register(ToCobs::new())?;
    registry.register(CitrixCtx1Decode::new())?;
    registry.register(CitrixCtx1Encode::new())?;
    registry.register(FromDecimal::new())?;
    registry.register(ToDecimal::new())?;
    registry.register(FromFloat::new())?;
    registry.register(ToFloat::new())?;
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
    registry.register(DecodeNetbiosName::new())?;
    registry.register(EncodeNetbiosName::new())?;
    registry.register(FromOctal::new())?;
    registry.register(ToOctal::new())?;
    registry.register(FromQuotedPrintable::new())?;
    registry.register(ToQuotedPrintable::new())?;
    registry.register(Rot13::new())?;
    registry.register(UnicodeTextFormat::new())?;
    registry.register(UrlDecode::new())?;
    registry.register(UrlEncode::new())?;
    registry.register(VarIntDecode::new())?;
    registry.register(VarIntEncode::new())?;
    registry.register(EscapeUnicodeCharacters::new())?;
    registry.register(UnescapeUnicodeCharacters::new())?;

    // 62 is not a power of two, so Base62 reads the whole input as one integer
    // rather than as a stream of bit groups — which is why it needs arbitrary
    // precision and the rest of this family does not.
    #[cfg(feature = "bignum")]
    {
        registry.register(FromBase62::new())?;
        registry.register(ToBase62::new())?;
    }
    Ok(())
}

/// Pulling indicators out of text, and putting them back safely.
#[cfg_attr(
    not(feature = "text"),
    expect(unused_variables, reason = "the text pack is not enabled")
)]
fn register_extractors(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    #[cfg(feature = "text")]
    {
        registry.register(ExtractDomains::new())?;
        registry.register(ExtractEmailAddresses::new())?;
        registry.register(ExtractFilePaths::new())?;
        registry.register(ExtractHashes::new())?;
        registry.register(ExtractIpAddresses::new())?;
        registry.register(ExtractMacAddresses::new())?;
        registry.register(ExtractUrls::new())?;
        registry.register(Strings::new())?;
        registry.register(DefangIpAddresses::new())?;
        registry.register(DefangUrl::new())?;
        registry.register(FangUrl::new())?;
    }
    Ok(())
}

/// Splitting a recipe and rejoining it.
fn register_flow(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Fork::new())?;
    registry.register(Merge::new())?;
    registry.register(Comment::new())?;
    Ok(())
}

/// Digests and message authentication.
#[cfg_attr(
    not(feature = "hash"),
    expect(unused_variables, reason = "the hash pack is not enabled")
)]
fn register_hashing(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    // SHA-0 and MurmurHash3 need no dependency, so they are not behind the
    // `hash` pack. That pack exists to keep the RustCrypto tree optional, and
    // gating a self-contained implementation behind it would charge these two
    // for a cost they do not incur.
    registry.register(Sha0::new())?;
    registry.register(MurmurHash3::new())?;

    #[cfg(feature = "hash")]
    {
        registry.register(Md5::new())?;
        registry.register(Sha1::new())?;
        registry.register(Sha2::new())?;
        registry.register(Sha3::new())?;
        registry.register(Keccak::new())?;
        registry.register(Shake::new())?;
        registry.register(FixedDigest::md2())?;
        registry.register(FixedDigest::md4())?;
        registry.register(FixedDigest::sm3())?;
        registry.register(FixedDigest::whirlpool())?;
        registry.register(NtHash::new())?;
        registry.register(Ripemd::new())?;
        registry.register(Hmac::new())?;
    }
    Ok(())
}

/// Bit-level logic, arithmetic, shifts, and rotations.
///
/// XOR and its brute force are here rather than with the ciphers. XOR against
/// a repeating key is a cipher in the sense that people use it as one, and a
/// bitwise operation in the sense that matters to a port — the catalog calls
/// it Logic, and that is the placement this follows.
fn register_logic(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(Bitwise::add())?;
    registry.register(Bitwise::and())?;
    registry.register(Bitwise::not())?;
    registry.register(Bitwise::or())?;
    registry.register(Bitwise::sub())?;
    registry.register(BitShift::left())?;
    registry.register(BitShift::right())?;
    registry.register(Rotate::left())?;
    registry.register(Rotate::right())?;
    registry.register(Ror13::new())?;
    registry.register(ParityBit::new())?;
    registry.register(Xor::new())?;

    #[cfg(feature = "analysis")]
    {
        registry.register(XorBruteForce::new())?;
    }
    Ok(())
}

/// Reading a structured format rather than transforming bytes.
fn register_parsing(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(ParseColourCode::new())?;
    registry.register(HexToPem::new())?;
    registry.register(PemToHex::new())?;
    registry.register(FormatMacAddresses::new())?;
    registry.register(StripHeader::ipv4())?;
    registry.register(StripHeader::tcp())?;
    registry.register(StripHeader::udp())?;
    registry.register(ParseUnixFilePermissions::new())?;

    // Object identifiers need arbitrary precision: an arc has no bound, and a
    // registered one really does exceed sixty-four bits.
    #[cfg(feature = "bignum")]
    {
        registry.register(HexToObjectIdentifier::new())?;
        registry.register(ObjectIdentifierToHex::new())?;
    }
    Ok(())
}

/// Comparisons over lists: what two sets share, how far two strings are apart.
fn register_sets(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(SetOperation::cartesian_product())?;
    registry.register(SetOperation::difference())?;
    registry.register(SetOperation::intersection())?;
    registry.register(SetOperation::symmetric_difference())?;
    registry.register(SetOperation::union())?;
    registry.register(PowerSet::new())?;
    registry.register(HammingDistance::new())?;
    registry.register(LevenshteinDistance::new())?;
    Ok(())
}

/// Reading, reshaping, and re-casing text.
fn register_text(registry: &mut OperationRegistry) -> Result<(), RegistryError> {
    registry.register(ToTable::new())?;
    registry.register(ToLowerCase::new())?;
    registry.register(ToUpperCase::new())?;
    registry.register(SwapCase::new())?;
    registry.register(AlternatingCaps::new())?;
    registry.register(GetAllCasings::new())?;
    registry.register(AddLineNumbers::new())?;
    registry.register(PadLines::new())?;
    registry.register(RemoveLineNumbers::new())?;
    registry.register(RemoveWhitespace::new())?;
    registry.register(Tail::new())?;
    registry.register(RemoveAnsiEscapeCodes::new())?;
    registry.register(StripHttpHeaders::new())?;
    registry.register(DechunkHttpResponse::new())?;
    registry.register(Wrap::new())?;
    registry.register(Split::new())?;
    registry.register(Unique::new())?;
    registry.register(ExpandAlphabetRange::new())?;
    registry.register(FromCaseInsensitiveRegex::new())?;
    registry.register(EscapeSmartCharacters::new())?;
    registry.register(StripHtmlTags::new())?;
    registry.register(HtmlToText::new())?;
    registry.register(UnescapeString::new())?;
    registry.register(GenerateDeBruijnSequence::new())?;
    registry.register(XkcdRandomNumber::new())?;
    registry.register(ToCaseInsensitiveRegex::new())?;

    #[cfg(feature = "text")]
    {
        registry.register(FindReplace::new())?;
        registry.register(CountOccurrences::new())?;
    }
    Ok(())
}
