//! AES-CBC / CFB / OFB / CTR / ECB / GCM for `CyberChef` 11.3 modes.
//!
//! This module owns argument validation and mode dispatch only; the cipher
//! implementations live in [`super::padded`], [`super::stream`], and
//! [`super::gcm`].

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use super::failure::{INVALID_IV, INVALID_KEY, INVALID_LENGTH, INVALID_MODE};
use super::gcm::{decrypt_gcm, encrypt_gcm};
use super::padded::{decrypt_cbc, decrypt_ecb, encrypt_cbc, encrypt_ecb};
use super::stream::{crypt_ctr, crypt_ofb, decrypt_cfb, encrypt_cfb};
use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const BLOCK_BYTES: usize = 16;

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
    if no_padding && !input.len().is_multiple_of(BLOCK_BYTES) {
        return Err(failed(INVALID_LENGTH));
    }
    let (mut body, tag) = match mode {
        "CBC" => (encrypt_cbc(input, params.key, &iv, no_padding)?, None),
        "CFB" => (encrypt_cfb(input, params.key, &iv)?, None),
        "OFB" => (crypt_ofb(input, params.key, &iv)?, None),
        "CTR" => (crypt_ctr(input, params.key, &iv)?, None),
        "ECB" => (encrypt_ecb(input, params.key, no_padding)?, None),
        "GCM" => {
            let (cipher, tag) = encrypt_gcm(input, params.key, &iv, params.aad)?;
            (cipher, Some(tag))
        }
        _ => return Err(failed(INVALID_MODE)),
    };
    body = attach_iv(body, iv, params.include_iv)?;
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
        "CFB" => decrypt_cfb(input, params.key, &iv)?,
        "OFB" => crypt_ofb(input, params.key, &iv)?,
        "CTR" => crypt_ctr(input, params.key, &iv)?,
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
        ("Hex", Some(tag)) => Ok(with_tag(to_hex_lower(body), &to_hex_lower(tag))),
        ("Hex", None) => Ok(to_hex_lower(body)),
        ("Raw", Some(tag)) => Ok(with_tag(latin1(body), &latin1(tag))),
        _ => Err(failed(INVALID_MODE)),
    }
}

/// Appends the reference's `\n\nTag: <value>` trailer.
fn with_tag(mut body: String, tag: &str) -> String {
    body.push_str("\n\nTag: ");
    body.push_str(tag);
    body
}

/// Places the IV before or after the ciphertext, or leaves it out.
fn attach_iv(body: Vec<u8>, iv: Vec<u8>, include_iv: &str) -> Result<Vec<u8>, OperationError> {
    match include_iv {
        "Prepend" => {
            let mut out = iv;
            out.extend_from_slice(&body);
            Ok(out)
        }
        "Append" => {
            let mut out = body;
            out.extend_from_slice(&iv);
            Ok(out)
        }
        "Off" => Ok(body),
        _ => Err(failed(INVALID_MODE)),
    }
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn parse_mode(mode: &str) -> Result<(&str, bool), OperationError> {
    if let Some(base) = mode.strip_suffix("/NoPadding") {
        if matches!(base, "CBC" | "ECB") {
            Ok((base, true))
        } else {
            Err(failed(INVALID_MODE))
        }
    } else if matches!(mode, "CBC" | "CFB" | "OFB" | "CTR" | "ECB" | "GCM") {
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

/// An empty IV becomes sixteen null bytes, matching the reference; GCM also
/// accepts a twelve-byte nonce.
fn normalize_iv(iv: &[u8], mode: &str) -> Result<Vec<u8>, OperationError> {
    if iv.is_empty() {
        return Ok(alloc::vec![0; BLOCK_BYTES]);
    }
    let accepted = if mode == "GCM" {
        matches!(iv.len(), 12 | 16)
    } else {
        iv.len() == BLOCK_BYTES
    };
    if accepted {
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
