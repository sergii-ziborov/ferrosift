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
mod bitwise;
mod bytes;
mod charcode;
mod checksum;
mod classical;
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
mod jsstr;
mod key;
mod lines;
mod modhex;
mod morse;
mod octal;
mod registry;
mod rot13;
mod sift;
mod spec;
mod url;
mod value;
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
pub use bitwise::{BitShift, Bitwise, Ror13, Rotate, SwapEndianness};
pub use bytes::{DropBytes, TakeBytes};
pub use charcode::{FromCharcode, ToCharcode};
pub use checksum::{Checksum, LuhnChecksum};
pub use classical::{ClassicalCipher, Rot47};
pub use decimal::{FromDecimal, ToDecimal};
pub use flow::{Fork, Merge};
pub use head::Head;
pub use hex::{FromHex, ToHex};
pub use hexdump::{FromHexdump, ToHexdump};
pub use html::{FromHtmlEntity, ToHtmlEntity};
pub use identity::Identity;
pub use lines::{AddLineNumbers, PadLines, RemoveLineNumbers, Tail};
pub use modhex::{FromModhex, ToModhex};
pub use morse::{FromMorseCode, ToMorseCode};
pub use octal::{FromOctal, ToOctal};
pub use rot13::Rot13;
pub use sift::{DropNthBytes, RemoveNullBytes, RemoveWhitespace, Reverse, TakeNthBytes};
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

pub use registry::default_registry;
