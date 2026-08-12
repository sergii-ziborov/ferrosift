//! AES-CBC / AES-ECB / AES-GCM for `CyberChef` 11.3 modes.

use alloc::string::String;
use alloc::vec::Vec;

use aes::Aes128;
use aes::Aes192;
use aes::Aes256;
use aes_gcm::aead::generic_array::typenum::{U12, U16};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{AesGcm, Nonce};
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cipher::block_padding::{NoPadding, Pkcs7};
use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const INVALID_KEY: &str = "crypto.aes.invalid_key_length";
const INVALID_MODE: &str = "crypto.aes.invalid_mode";
const INVALID_LENGTH: &str = "crypto.aes.invalid_length";
const DECRYPT_FAILED: &str = "crypto.aes.decrypt_failed";
const INVALID_IV: &str = "crypto.aes.invalid_iv";

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes192CbcEnc = cbc::Encryptor<Aes192>;
type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;
type Aes192CbcDec = cbc::Decryptor<Aes192>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;
type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes192EcbEnc = ecb::Encryptor<Aes192>;
type Aes256EcbEnc = ecb::Encryptor<Aes256>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;
type Aes192EcbDec = ecb::Decryptor<Aes192>;
type Aes256EcbDec = ecb::Decryptor<Aes256>;
type Aes128Gcm12 = AesGcm<Aes128, U12>;
type Aes192Gcm12 = AesGcm<Aes192, U12>;
type Aes256Gcm12 = AesGcm<Aes256, U12>;
type Aes128Gcm16 = AesGcm<Aes128, U16>;
type Aes192Gcm16 = AesGcm<Aes192, U16>;
type Aes256Gcm16 = AesGcm<Aes256, U16>;

pub(super) struct EncryptParams<'a> {
    pub key: &'a [u8],
    pub iv: &'a [u8],
    pub mode: &'a str,
    pub aad: &'a [u8],
    pub include_iv: &'a str,
}

pub(super) struct DecryptParams<'a> {
    pub key: &'a [u8],
    pub iv: &'a [u8],
    pub mode: &'a str,
    pub tag: &'a [u8],
    pub aad: &'a [u8],
}

pub(super) fn encrypt(
    input: &[u8],
    params: &EncryptParams<'_>,
    context: &OperationContext<'_>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), OperationError> {
    context.ensure_active()?;
    validate_key(params.key)?;
    let (mode, no_padding) = parse_mode(params.mode)?;
    let iv = normalize_iv(params.iv, mode)?;
    if no_padding && !input.len().is_multiple_of(16) {
        return Err(failed(INVALID_LENGTH));
    }
    let (mut body, tag) = match mode {
        "CBC" => (encrypt_cbc(input, params.key, &iv, no_padding)?, None),
        "ECB" => (encrypt_ecb(input, params.key, no_padding)?, None),
        "GCM" => {
            let (cipher, tag) = encrypt_gcm(input, params.key, &iv, params.aad)?;
            (cipher, Some(tag))
        }
        _ => return Err(failed(INVALID_MODE)),
    };
    match params.include_iv {
        "Prepend" => {
            let mut out = iv;
            out.extend_from_slice(&body);
            body = out;
        }
        "Append" => body.extend_from_slice(&iv),
        "Off" => {}
        _ => return Err(failed(INVALID_MODE)),
    }
    ensure_budget(body.len(), context)?;
    context.ensure_active()?;
    Ok((body, tag))
}

pub(super) fn decrypt(
    input: &[u8],
    params: &DecryptParams<'_>,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    validate_key(params.key)?;
    let (mode, no_padding) = parse_mode(params.mode)?;
    let iv = normalize_iv(params.iv, mode)?;
    let plain = match mode {
        "CBC" => decrypt_cbc(input, params.key, &iv, no_padding)?,
        "ECB" => decrypt_ecb(input, params.key, no_padding)?,
        "GCM" => decrypt_gcm(input, params.key, &iv, params.tag, params.aad)?,
        _ => return Err(failed(INVALID_MODE)),
    };
    ensure_budget(plain.len(), context)?;
    context.ensure_active()?;
    Ok(plain)
}

pub(super) fn format_encrypt_output(
    body: &[u8],
    tag: Option<&[u8]>,
    output_format: &str,
) -> Result<String, OperationError> {
    match (output_format, tag) {
        ("Hex", Some(tag)) => {
            let mut text = to_hex_lower(body);
            text.push_str("\n\nTag: ");
            text.push_str(&to_hex_lower(tag));
            Ok(text)
        }
        ("Hex", None) => Ok(to_hex_lower(body)),
        ("Raw", Some(tag)) => {
            let mut out = latin1(body);
            out.push_str("\n\nTag: ");
            out.push_str(&latin1(tag));
            Ok(out)
        }
        _ => Err(failed(INVALID_MODE)),
    }
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn parse_mode(mode: &str) -> Result<(&str, bool), OperationError> {
    if let Some(base) = mode.strip_suffix("/NoPadding") {
        Ok((base, true))
    } else if matches!(mode, "CBC" | "ECB" | "GCM") {
        Ok((mode, false))
    } else {
        Err(failed(INVALID_MODE))
    }
}

fn validate_key(key: &[u8]) -> Result<(), OperationError> {
    if matches!(key.len(), 16 | 24 | 32) {
        Ok(())
    } else {
        Err(failed(INVALID_KEY))
    }
}

fn normalize_iv(iv: &[u8], mode: &str) -> Result<Vec<u8>, OperationError> {
    if iv.is_empty() {
        return Ok(alloc::vec![0; 16]);
    }
    let ok = if mode == "GCM" {
        matches!(iv.len(), 12 | 16)
    } else {
        iv.len() == 16
    };
    if ok {
        Ok(iv.to_vec())
    } else {
        Err(failed(INVALID_IV))
    }
}

fn ensure_budget(len: usize, context: &OperationContext<'_>) -> Result<(), OperationError> {
    if u64::try_from(len).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}

fn fail_len<E>(_: E) -> OperationError {
    failed(INVALID_LENGTH)
}

fn fail_dec<E>(_: E) -> OperationError {
    failed(DECRYPT_FAILED)
}

fn encrypt_cbc(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = if no_padding {
        input.to_vec()
    } else {
        let mut buf = input.to_vec();
        buf.resize(input.len() + 16, 0);
        buf
    };
    let out = match (key.len(), no_padding) {
        (16, true) => Aes128CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (24, true) => Aes192CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (32, true) => Aes256CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (16, false) => Aes128CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (24, false) => Aes192CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (32, false) => Aes256CbcEnc::new(key.into(), iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        _ => return Err(failed(INVALID_KEY)),
    };
    Ok(out)
}

fn decrypt_cbc(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    let out = match (key.len(), no_padding) {
        (16, true) => Aes128CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (24, true) => Aes192CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (32, true) => Aes256CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (16, false) => Aes128CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (24, false) => Aes192CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (32, false) => Aes256CbcDec::new(key.into(), iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        _ => return Err(failed(INVALID_KEY)),
    };
    Ok(out)
}

fn encrypt_ecb(input: &[u8], key: &[u8], no_padding: bool) -> Result<Vec<u8>, OperationError> {
    use cipher::KeyInit;
    let mut buffer = if no_padding {
        input.to_vec()
    } else {
        let mut buf = input.to_vec();
        buf.resize(input.len() + 16, 0);
        buf
    };
    let out = match (key.len(), no_padding) {
        (16, true) => Aes128EcbEnc::new(key.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (24, true) => Aes192EcbEnc::new(key.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (32, true) => Aes256EcbEnc::new(key.into())
            .encrypt_padded_mut::<NoPadding>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (16, false) => Aes128EcbEnc::new(key.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (24, false) => Aes192EcbEnc::new(key.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        (32, false) => Aes256EcbEnc::new(key.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, input.len())
            .map_err(fail_len)?
            .to_vec(),
        _ => return Err(failed(INVALID_KEY)),
    };
    Ok(out)
}

fn decrypt_ecb(input: &[u8], key: &[u8], no_padding: bool) -> Result<Vec<u8>, OperationError> {
    use cipher::KeyInit;
    let mut buffer = input.to_vec();
    let out = match (key.len(), no_padding) {
        (16, true) => Aes128EcbDec::new(key.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (24, true) => Aes192EcbDec::new(key.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (32, true) => Aes256EcbDec::new(key.into())
            .decrypt_padded_mut::<NoPadding>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (16, false) => Aes128EcbDec::new(key.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (24, false) => Aes192EcbDec::new(key.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        (32, false) => Aes256EcbDec::new(key.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(fail_dec)?
            .to_vec(),
        _ => return Err(failed(INVALID_KEY)),
    };
    Ok(out)
}

fn encrypt_gcm(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), OperationError> {
    let payload = Payload { msg: input, aad };
    let sealed = match (key.len(), iv.len()) {
        (16, 12) => Aes128Gcm12::new(key.into())
            .encrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_len)?,
        (16, 16) => Aes128Gcm16::new(key.into())
            .encrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_len)?,
        (24, 12) => Aes192Gcm12::new(key.into())
            .encrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_len)?,
        (24, 16) => Aes192Gcm16::new(key.into())
            .encrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_len)?,
        (32, 12) => Aes256Gcm12::new(key.into())
            .encrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_len)?,
        (32, 16) => Aes256Gcm16::new(key.into())
            .encrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_len)?,
        _ => return Err(failed(INVALID_KEY)),
    };
    if sealed.len() < 16 {
        return Err(failed(INVALID_LENGTH));
    }
    let split = sealed.len() - 16;
    Ok((sealed[..split].to_vec(), sealed[split..].to_vec()))
}

fn decrypt_gcm(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    tag: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, OperationError> {
    if tag.len() != 16 {
        return Err(failed(DECRYPT_FAILED));
    }
    let mut sealed = input.to_vec();
    sealed.extend_from_slice(tag);
    let payload = Payload { msg: &sealed, aad };
    match (key.len(), iv.len()) {
        (16, 12) => Aes128Gcm12::new(key.into())
            .decrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_dec),
        (16, 16) => Aes128Gcm16::new(key.into())
            .decrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_dec),
        (24, 12) => Aes192Gcm12::new(key.into())
            .decrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_dec),
        (24, 16) => Aes192Gcm16::new(key.into())
            .decrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_dec),
        (32, 12) => Aes256Gcm12::new(key.into())
            .decrypt(Nonce::<U12>::from_slice(iv), payload)
            .map_err(fail_dec),
        (32, 16) => Aes256Gcm16::new(key.into())
            .decrypt(Nonce::<U16>::from_slice(iv), payload)
            .map_err(fail_dec),
        _ => Err(failed(INVALID_KEY)),
    }
}
