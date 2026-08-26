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
mod bifid;
mod base45;
mod base58;
#[cfg(feature = "bignum")]
mod base62;
mod base64;
mod base85;
mod base92;
mod binary;
mod bitwise;
mod braille;
mod brute;
mod bytes;
mod casing;
mod charcode;
mod checksum;
mod classical;
mod cobs;
mod ctx1;
mod decimal;
mod failure;
mod float;
mod flow;
mod generate;
mod head;
mod hex;
mod hex_util;
mod bech32;
mod ls47;
mod msscript;
mod offsetcheck;
mod stats;
mod caseregex;
mod comment;
mod punycode;
mod unixperms;
mod hexcontent;
mod hexdump;
mod html;
mod identity;
mod jscompat;
mod key;
mod lines;
mod lists;
mod markup;
mod misc;
mod modhex;
mod legacy;
mod morse;
mod netfmt;
mod netstrip;
mod octal;
#[cfg(feature = "bignum")]
mod oid;
mod pem;
mod quoted_printable;
mod registry;
mod rot13;
mod sets;
mod shape;
mod sift;
mod spec;
mod substitute;
mod unicode_escape;
mod url;
mod value;
mod varint;
mod xkcd;
mod xor;

#[cfg(feature = "crypto")]
mod aes_kw;
#[cfg(feature = "crypto")]
mod aes_op;
#[cfg(feature = "arithmetic")]
mod bigint;
#[cfg(feature = "crypto")]
mod codec_bytes;
#[cfg(feature = "compression")]
mod compress;
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
#[cfg(feature = "crypto")]
mod rc4_op;
#[cfg(feature = "analysis")]
mod suggest;
#[cfg(feature = "analysis")]
mod xor_brute;

pub use annotate::HtmlToText;
pub use base32::{FromBase32, ToBase32};
pub use base45::{FromBase45, ToBase45};
pub use bacon::{BaconDecode, BaconEncode};
pub use bifid::BifidCipher;
pub use xkcd::XkcdRandomNumber;
pub use base58::{FromBase58, ToBase58};
#[cfg(feature = "bignum")]
pub use base62::{FromBase62, ToBase62};
pub use base64::{FromBase64, ToBase64};
pub use base85::{FromBase85, ToBase85};
pub use base92::{FromBase92, ToBase92};
pub use binary::{FromBinary, ToBinary};
pub use bitwise::{BitShift, Bitwise, Ror13, Rotate, SwapEndianness};
pub use braille::{FromBraille, ToBraille, UnicodeTextFormat};
pub use brute::{Rot13BruteForce, Rot47BruteForce};
pub use bytes::{DropBytes, TakeBytes};
pub use casing::{AlternatingCaps, GetAllCasings, SwapCase, ToLowerCase, ToUpperCase};
pub use charcode::{FromCharcode, ToCharcode};
pub use checksum::{Checksum, LuhnChecksum};
pub use classical::{ClassicalCipher, Rot47};
pub use cobs::{FromCobs, ToCobs};
pub use ctx1::{CitrixCtx1Decode, CitrixCtx1Encode};
pub use decimal::{FromDecimal, ToDecimal};
pub use float::{FromFloat, ToFloat};
pub use flow::{Fork, Merge};
pub use generate::GenerateDeBruijnSequence;
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use bech32::{FromBech32, ToBech32};
pub use ls47::{Ls47Decrypt, Ls47Encrypt};
pub use msscript::MicrosoftScriptDecoder;
pub use offsetcheck::OffsetChecker;
pub use stats::{ChiSquare, IndexOfCoincidence};
pub use caseregex::ToCaseInsensitiveRegex;
pub use punycode::Punycode;
pub use unixperms::ParseUnixFilePermissions;
pub use comment::Comment;
pub use hexcontent::{FromHexContent, ToHexContent};
pub use legacy::{MurmurHash3, Sha0};
pub use netstrip::StripHeader;
pub use hexdump::{FromHexdump, ToHexdump};
pub use html::{FromHtmlEntity, ToHtmlEntity};
pub use identity::Identity;
pub use lines::{AddLineNumbers, PadLines, RemoveLineNumbers, Tail};
pub use lists::{Split, Unique};
pub use markup::{EscapeSmartCharacters, StripHtmlTags};
pub use misc::{CaretMDecode, FromCaseInsensitiveRegex, PowerSet};
pub use modhex::{FromModhex, ToModhex};
pub use morse::{FromMorseCode, ToMorseCode};
pub use netfmt::{FormatMacAddresses, ParityBit};
pub use octal::{FromOctal, ToOctal};
#[cfg(feature = "bignum")]
pub use oid::{HexToObjectIdentifier, ObjectIdentifierToHex};
pub use pem::{HexToPem, PemToHex};
pub use quoted_printable::{FromQuotedPrintable, ToQuotedPrintable};
pub use rot13::Rot13;
pub use sets::{HammingDistance, LevenshteinDistance, SetOperation};
pub use shape::{
    DechunkHttpResponse, ExpandAlphabetRange, RemoveAnsiEscapeCodes, StripHttpHeaders, Wrap,
};
pub use sift::{DropNthBytes, RemoveNullBytes, RemoveWhitespace, Reverse, TakeNthBytes};
pub use substitute::{Substitute, UnescapeString};
pub use unicode_escape::{
    DecodeNetbiosName, EncodeNetbiosName, EscapeUnicodeCharacters, UnescapeUnicodeCharacters,
};
pub use url::{UrlDecode, UrlEncode};
pub use varint::{VarIntDecode, VarIntEncode};
pub use xor::Xor;

#[cfg(feature = "crypto")]
pub use aes_kw::{AesKeyUnwrap, AesKeyWrap};
#[cfg(feature = "crypto")]
pub use aes_op::{AesDecrypt, AesEncrypt};
#[cfg(feature = "arithmetic")]
pub use bigint::{ExtendedGcd, ModularInverse};
#[cfg(feature = "compression")]
pub use compress::{
    Bzip2Compress, Bzip2Decompress, Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate,
};
#[cfg(feature = "text")]
pub use count::CountOccurrences;
#[cfg(feature = "text")]
pub use defang::{DefangIpAddresses, DefangUrl, FangUrl};
#[cfg(feature = "hash")]
pub use digest::{FixedDigest, Ripemd};
#[cfg(feature = "text")]
pub use extract::{
    ExtractDomains, ExtractEmailAddresses, ExtractFilePaths, ExtractHashes, ExtractIpAddresses,
    ExtractMacAddresses, ExtractUrls, Strings,
};
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
#[cfg(feature = "crypto")]
pub use rc4_op::{Rc4, Rc4Drop};
#[cfg(feature = "analysis")]
pub use suggest::SuggestRecipe;
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
