//! Stream and stream-like modes: CFB, OFB, and CTR.

use alloc::vec::Vec;

use aes::{Aes128, Aes192, Aes256};
use cipher::{AsyncStreamCipher, KeyIvInit, StreamCipher};
use ctr::Ctr128BE;
use ferrosift_core::OperationError;

use super::failure::invalid_key;

type Aes128CfbEnc = cfb_mode::Encryptor<Aes128>;
type Aes192CfbEnc = cfb_mode::Encryptor<Aes192>;
type Aes256CfbEnc = cfb_mode::Encryptor<Aes256>;
type Aes128CfbDec = cfb_mode::Decryptor<Aes128>;
type Aes192CfbDec = cfb_mode::Decryptor<Aes192>;
type Aes256CfbDec = cfb_mode::Decryptor<Aes256>;
type Aes128Ofb = ofb::Ofb<Aes128>;
type Aes192Ofb = ofb::Ofb<Aes192>;
type Aes256Ofb = ofb::Ofb<Aes256>;
type Aes128Ctr = Ctr128BE<Aes128>;
type Aes192Ctr = Ctr128BE<Aes192>;
type Aes256Ctr = Ctr128BE<Aes256>;

/// Dispatches an in-place transform over the three AES key sizes.
macro_rules! transform_in_place {
    ($key:expr, $iv:expr, $buffer:expr,
     $t128:ty, $t192:ty, $t256:ty, $method:ident) => {
        match $key.len() {
            16 => <$t128>::new($key.into(), $iv.into()).$method($buffer),
            24 => <$t192>::new($key.into(), $iv.into()).$method($buffer),
            32 => <$t256>::new($key.into(), $iv.into()).$method($buffer),
            _ => return Err(invalid_key()),
        }
    };
}

pub(super) fn encrypt_cfb(input: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    transform_in_place!(
        key,
        iv,
        &mut buffer,
        Aes128CfbEnc,
        Aes192CfbEnc,
        Aes256CfbEnc,
        encrypt
    );
    Ok(buffer)
}

pub(super) fn decrypt_cfb(input: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    transform_in_place!(
        key,
        iv,
        &mut buffer,
        Aes128CfbDec,
        Aes192CfbDec,
        Aes256CfbDec,
        decrypt
    );
    Ok(buffer)
}

/// OFB is its own inverse, so one direction serves both.
pub(super) fn crypt_ofb(input: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    transform_in_place!(
        key,
        iv,
        &mut buffer,
        Aes128Ofb,
        Aes192Ofb,
        Aes256Ofb,
        apply_keystream
    );
    Ok(buffer)
}

/// CTR is its own inverse, so one direction serves both.
pub(super) fn crypt_ctr(input: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    transform_in_place!(
        key,
        iv,
        &mut buffer,
        Aes128Ctr,
        Aes192Ctr,
        Aes256Ctr,
        apply_keystream
    );
    Ok(buffer)
}
