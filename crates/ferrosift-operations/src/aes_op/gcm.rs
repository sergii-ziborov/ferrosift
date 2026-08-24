//! Authenticated encryption: AES-GCM with 12- or 16-byte nonces.

use alloc::vec::Vec;

use aes::{Aes128, Aes192, Aes256};
use aes_gcm::aead::generic_array::typenum::{U12, U16};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{AesGcm, Nonce};
use ferrosift_core::OperationError;

use super::failure::{DECRYPT_FAILED, INVALID_LENGTH, fail_dec, fail_len, invalid_key};
use crate::failure::failed;

const TAG_BYTES: usize = 16;

type Aes128Gcm12 = AesGcm<Aes128, U12>;
type Aes192Gcm12 = AesGcm<Aes192, U12>;
type Aes256Gcm12 = AesGcm<Aes256, U12>;
type Aes128Gcm16 = AesGcm<Aes128, U16>;
type Aes192Gcm16 = AesGcm<Aes192, U16>;
type Aes256Gcm16 = AesGcm<Aes256, U16>;

/// Dispatches a GCM operation over the three key sizes and two nonce widths.
macro_rules! gcm_dispatch {
    ($key:expr, $iv:expr, $payload:expr, $method:ident, $map:expr) => {
        match ($key.len(), $iv.len()) {
            (16, 12) => Aes128Gcm12::new($key.into())
                .$method(Nonce::<U12>::from_slice($iv), $payload)
                .map_err($map),
            (16, 16) => Aes128Gcm16::new($key.into())
                .$method(Nonce::<U16>::from_slice($iv), $payload)
                .map_err($map),
            (24, 12) => Aes192Gcm12::new($key.into())
                .$method(Nonce::<U12>::from_slice($iv), $payload)
                .map_err($map),
            (24, 16) => Aes192Gcm16::new($key.into())
                .$method(Nonce::<U16>::from_slice($iv), $payload)
                .map_err($map),
            (32, 12) => Aes256Gcm12::new($key.into())
                .$method(Nonce::<U12>::from_slice($iv), $payload)
                .map_err($map),
            (32, 16) => Aes256Gcm16::new($key.into())
                .$method(Nonce::<U16>::from_slice($iv), $payload)
                .map_err($map),
            _ => Err(invalid_key()),
        }
    };
}

/// Encrypts and splits the trailing authentication tag from the ciphertext.
pub(super) fn encrypt_gcm(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), OperationError> {
    let payload = Payload { msg: input, aad };
    let sealed = gcm_dispatch!(key, iv, payload, encrypt, fail_len)?;
    if sealed.len() < TAG_BYTES {
        return Err(failed(INVALID_LENGTH));
    }
    let split = sealed.len() - TAG_BYTES;
    Ok((sealed[..split].to_vec(), sealed[split..].to_vec()))
}

/// Re-attaches the tag before verifying and decrypting.
pub(super) fn decrypt_gcm(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    tag: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, OperationError> {
    if tag.len() != TAG_BYTES {
        return Err(failed(DECRYPT_FAILED));
    }
    let mut sealed = input.to_vec();
    sealed.extend_from_slice(tag);
    let payload = Payload { msg: &sealed, aad };
    gcm_dispatch!(key, iv, payload, decrypt, fail_dec)
}
