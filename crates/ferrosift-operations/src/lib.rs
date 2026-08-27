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
mod annotate;
mod args;
mod bacon;
mod base32;
mod base45;
mod base58;
#[cfg(feature = "bignum")]
mod base62;
mod base64;
mod base85;
mod base92;
mod bcd;
mod bcrypt_parse;
mod bech32;
mod bifid;
mod binary;
mod bitwise;
mod braille;
mod brute;
mod bytes;
mod caseregex;
mod casing;
mod charcode;
mod checksum;
mod classical;
mod cobs;
mod colour;
mod comment;
mod ctx1;
mod decimal;
mod failure;
mod float;
mod flow;
mod generate;
mod head;
mod hex;
mod hex_util;
mod hexcontent;
mod hexdump;
mod html;
mod identity;
mod ipformat;
mod jscompat;
mod key;
mod label;
mod legacy;
mod lines;
mod lists;
mod ls47;
mod lznt1;
mod markup;
mod misc;
mod modhex;
mod morse;
mod msscript;
mod netfmt;
mod netstrip;
mod octal;
mod offsetcheck;
#[cfg(feature = "bignum")]
mod oid;
mod pem;
mod punycode;
mod quoted_printable;
mod registry;
mod rot13;
mod sets;
mod shape;
mod sift;
mod spec;
mod stats;
mod substitute;
mod table;
mod tea;
mod tlv;
mod unicode_escape;
mod unixperms;
mod url;
mod value;
mod varint;
mod xkcd;
mod xor;
mod xxtea;

#[cfg(feature = "crypto")]
mod aes_kw;
#[cfg(feature = "crypto")]
mod aes_op;
#[cfg(feature = "arithmetic")]
mod arith;
#[cfg(feature = "arithmetic")]
mod bigint;
#[cfg(feature = "crypto")]
mod codec_bytes;
#[cfg(feature = "compression")]
mod compress;
#[cfg(feature = "arithmetic")]
mod convert;
#[cfg(feature = "text")]
mod count;
#[cfg(feature = "compression")]
mod crc32;
#[cfg(feature = "text")]
mod defang;
#[cfg(feature = "hash")]
mod digest;
#[cfg(feature = "text")]
mod extract;
#[cfg(feature = "bignum")]
mod filetime;
#[cfg(feature = "text")]
mod find;
#[cfg(feature = "hash")]
mod hash;
#[cfg(feature = "hash")]
mod hmac_op;
#[cfg(feature = "crypto")]
mod kdf;
#[cfg(feature = "hash")]
mod nthash;
#[cfg(feature = "bignum")]
mod numbase;
#[cfg(feature = "crypto")]
mod rc4_op;
#[cfg(feature = "analysis")]
mod suggest;
#[cfg(feature = "bignum")]
mod textint;
#[cfg(feature = "analysis")]
mod xor_brute;

pub use annotate::HtmlToText;
pub use bacon::{BaconDecode, BaconEncode};
pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use base58::{FromBase58, ToBase58};
#[cfg(feature = "bignum")]
pub use base62::{FromBase62, ToBase62};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use base92::{FromBase92, ToBase92};
pub use bcd::{FromBcd, ToBcd};
pub use bcrypt_parse::BcryptParse;
pub use bech32::{FromBech32, ToBech32};
pub use bifid::BifidCipher;
pub use binary::{FromBinary, ToBinary};
pub use bitwise::{BitShift, Bitwise, Ror13, Rotate, SwapEndianness};
pub use braille::{FromBraille, ToBraille, UnicodeTextFormat};
pub use brute::{Rot13BruteForce, Rot47BruteForce};
pub use bytes::{DropBytes, TakeBytes};
pub use caseregex::ToCaseInsensitiveRegex;
pub use casing::{AlternatingCaps, GetAllCasings, SwapCase, ToLowerCase, ToUpperCase};
pub use charcode::{FromCharcode, ToCharcode};
pub use checksum::{Checksum, LuhnChecksum};
pub use classical::{ClassicalCipher, Rot47};
pub use cobs::{FromCobs, ToCobs};
pub use colour::ParseColourCode;
pub use comment::Comment;
pub use ctx1::{CitrixCtx1Decode, CitrixCtx1Encode};
pub use decimal::{FromDecimal, ToDecimal};
pub use float::{FromFloat, ToFloat};
pub use flow::{Fork, Merge};
pub use generate::GenerateDeBruijnSequence;
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use hexcontent::{FromHexContent, ToHexContent};
pub use hexdump::{FromHexdump, ToHexdump};
pub use html::{FromHtmlEntity, ToHtmlEntity};
pub use identity::Identity;
pub use ipformat::ChangeIpFormat;
pub use label::Label;
pub use legacy::{MurmurHash3, Sha0};
pub use lines::{AddLineNumbers, PadLines, RemoveLineNumbers, Tail};
pub use lists::{Split, Unique};
pub use ls47::{Ls47Decrypt, Ls47Encrypt};
pub use lznt1::Lznt1Decompress;
pub use markup::{EscapeSmartCharacters, StripHtmlTags};
pub use misc::{CaretMDecode, FromCaseInsensitiveRegex, PowerSet};
pub use modhex::{FromModhex, ToModhex};
pub use morse::{FromMorseCode, ToMorseCode};
pub use msscript::MicrosoftScriptDecoder;
pub use netfmt::{FormatMacAddresses, ParityBit};
pub use netstrip::StripHeader;
pub use octal::{FromOctal, ToOctal};
pub use offsetcheck::OffsetChecker;
#[cfg(feature = "bignum")]
pub use oid::{HexToObjectIdentifier, ObjectIdentifierToHex};
pub use pem::{HexToPem, PemToHex};
pub use punycode::Punycode;
pub use quoted_printable::{FromQuotedPrintable, ToQuotedPrintable};
pub use rot13::Rot13;
pub use sets::{HammingDistance, LevenshteinDistance, SetOperation};
pub use shape::{
    DechunkHttpResponse, ExpandAlphabetRange, RemoveAnsiEscapeCodes, StripHttpHeaders, Wrap,
};
pub use sift::{DropNthBytes, RemoveNullBytes, RemoveWhitespace, Reverse, TakeNthBytes};
pub use stats::{ChiSquare, IndexOfCoincidence};
pub use substitute::{Substitute, UnescapeString};
pub use table::ToTable;
pub use tea::Tea;
pub use tlv::ParseTlv;
pub use unicode_escape::{
    DecodeNetbiosName, EncodeNetbiosName, EscapeUnicodeCharacters, UnescapeUnicodeCharacters,
};
pub use unixperms::ParseUnixFilePermissions;
pub use url::{UrlDecode, UrlEncode};
pub use varint::{VarIntDecode, VarIntEncode};
pub use xkcd::XkcdRandomNumber;
pub use xor::Xor;
pub use xxtea::Xxtea;

#[cfg(feature = "crypto")]
pub use aes_kw::{AesKeyUnwrap, AesKeyWrap};
#[cfg(feature = "crypto")]
pub use aes_op::{AesDecrypt, AesEncrypt};
#[cfg(feature = "arithmetic")]
pub use arith::{Aggregate, Mod};
#[cfg(feature = "arithmetic")]
pub use bigint::{ExtendedGcd, ModularInverse};
#[cfg(feature = "compression")]
pub use compress::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
#[cfg(feature = "arithmetic")]
pub use convert::ConvertUnits;
#[cfg(feature = "text")]
pub use count::CountOccurrences;
#[cfg(feature = "text")]
pub use defang::{DefangIpAddresses, DefangUrl, FangUrl};
#[cfg(feature = "hash")]
pub use digest::{Blake2, Blake3, FixedDigest, Ripemd, Streebog};
#[cfg(feature = "text")]
pub use extract::{
    ExtractDomains, ExtractEmailAddresses, ExtractFilePaths, ExtractHashes, ExtractIpAddresses,
    ExtractMacAddresses, ExtractUrls, Strings,
};
#[cfg(feature = "bignum")]
pub use filetime::{FiletimeToUnix, UnixToFiletime};
#[cfg(feature = "text")]
pub use find::FindReplace;
#[cfg(feature = "hash")]
pub use hash::{Keccak, Md5, Sha1, Sha2, Sha3, Shake};
#[cfg(feature = "hash")]
pub use hmac_op::Hmac;
#[cfg(feature = "crypto")]
pub use kdf::{DerivePbkdf2Key, Scrypt};
#[cfg(feature = "hash")]
pub use nthash::NtHash;
#[cfg(feature = "bignum")]
pub use numbase::{FromBase, ToBase};
#[cfg(feature = "crypto")]
pub use rc4_op::{Rc4, Rc4Drop};
#[cfg(feature = "analysis")]
pub use suggest::SuggestRecipe;
#[cfg(feature = "bignum")]
pub use textint::TextIntegerConversion;
#[cfg(feature = "analysis")]
pub use xor_brute::XorBruteForce;

pub use registry::default_registry;

/// Test-only access to the registration families.
///
/// `tests/registry.rs` builds each family on its own and checks that every
/// operation in it declares a category the family accepts. Without that, a
/// family drifts into a junk drawer one convenient placement at a time — which
/// is exactly what happened to the old grouping.
///
/// Hidden from the documentation for the same reason as
/// [`jscompat_testing`]: the surface a caller uses is the operation catalog.
#[doc(hidden)]
pub mod registry_testing {
    use alloc::vec::Vec;

    use ferrosift_core::{OperationRegistry, RegistryError};

    /// One family: its name, the categories it accepts, and what it registers.
    pub struct Family {
        /// What to call this family when reporting a mismatch.
        pub name: &'static str,
        /// Catalog categories whose operations may live here.
        pub categories: &'static [&'static str],
        /// The display name and category of everything it registered.
        pub registered: Vec<(alloc::string::String, alloc::string::String)>,
    }

    /// Builds every family separately and reports what each one holds.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if a family's own registrations conflict.
    pub fn families() -> Result<Vec<Family>, RegistryError> {
        let mut result = Vec::new();
        for family in crate::registry::FAMILIES {
            let mut registry = OperationRegistry::new();
            (family.register)(&mut registry)?;
            result.push(Family {
                name: family.name,
                categories: family.categories,
                registered: registry
                    .catalog()
                    .map(|spec| (spec.display_name.clone(), spec.category.clone()))
                    .collect(),
            });
        }
        Ok(result)
    }
}

/// Test-only access to the JavaScript compatibility layer.
///
/// `tests/jscompat.rs` pins these against Node directly rather than only
/// through the operations that use them, which is what makes a divergence
/// legible: "our parseInt disagrees" instead of four unrelated operations
/// failing for reasons nobody would connect.
///
/// Hidden from the documentation because it is not part of the library's
/// contract — the surface a caller uses is the operation catalog.
#[doc(hidden)]
pub mod jscompat_testing {
    /// Arbitrary-precision arithmetic with the reference library's rules.
    ///
    /// Exposed so `tests/bignumber.rs` can replay the recorded answers
    /// directly, rather than only through whichever operations happen to use
    /// it -- an arithmetic rule nobody called would otherwise go unchecked.
    #[cfg(feature = "bignum")]
    pub use crate::jscompat::bignumber;

    use alloc::{string::String, vec::Vec};

    /// Whether JavaScript's `\s` matches this character.
    ///
    /// Wider than `char::is_whitespace`: it includes the byte-order mark.
    #[must_use]
    pub const fn is_js_whitespace(value: char) -> bool {
        crate::jscompat::delim::is_js_whitespace(value)
    }

    /// `String(x)` for a double.
    ///
    /// Exposed because the two notation thresholds are JavaScript's own, and
    /// a fixture generated by Node is the only honest way to check them.
    #[must_use]
    pub fn format_double(value: f64) -> String {
        crate::jscompat::double::format(value)
    }

    /// `parseInt`, as `Option` rather than the internal enum.
    ///
    /// `None` is JavaScript's `NaN`. The value saturates at a million, which
    /// is what every caller in this crate needs and no more.
    #[must_use]
    pub fn parse_int(token: &str, radix: u32) -> Option<i64> {
        match crate::jscompat::number::parse(token, radix) {
            crate::jscompat::number::JsInt::Nan => None,
            crate::jscompat::number::JsInt::Value(value) => Some(value),
        }
    }

    /// `parseInt` at radix ten, as the `Number` it really produces.
    ///
    /// Exposed separately from [`parse_int`] because the two answer different
    /// questions: that one classifies and this one is the value the reference
    /// goes on to print, digits and exponential form and all.
    #[must_use]
    pub fn parse_int_decimal(token: &str) -> f64 {
        crate::jscompat::number::parse_decimal(token)
    }

    /// `ToInt32`, which every JavaScript bitwise operator applies first.
    #[must_use]
    pub fn to_int32(value: f64) -> i32 {
        crate::jscompat::number::to_int32(value)
    }

    /// `ToUint8`, which storing into a `Uint8Array` or a `Buffer` applies.
    ///
    /// Exposed beside [`to_int32`] because the pair is the point: a
    /// toggleString field can hold a number no byte array can, and which of
    /// these two the consumer applies is what decides the answer. Checking them
    /// only through the operations that use them would have left the difference
    /// implicit in six places instead of pinned in one.
    #[must_use]
    pub fn to_uint8(value: f64) -> u8 {
        crate::jscompat::number::to_uint8(value)
    }

    /// Accumulates keys the way a JavaScript object literal used as a set does.
    #[derive(Default)]
    pub struct KeySet {
        inner: Vec<String>,
    }

    impl KeySet {
        /// Adds one key.
        pub fn insert(&mut self, key: &str) {
            let mut set = crate::jscompat::object::KeySet::new();
            for existing in &self.inner {
                set.insert(existing);
            }
            set.insert(key);
            self.inner = set.keys();
        }

        /// The keys in `Object.keys` order.
        #[must_use]
        pub fn into_keys(self) -> Vec<String> {
            self.inner
        }
    }
}
