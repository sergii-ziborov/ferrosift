//! Block modes that carry padding: CBC and ECB.

use alloc::vec::Vec;

use aes::{Aes128, Aes192, Aes256};
use cipher::block_padding::{NoPadding, Pkcs7};
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit};
use ferrosift_core::OperationError;

use super::failure::{fail_dec, fail_len, invalid_key};

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

/// Dispatches a padded encryption over the three AES key sizes.
///
/// The `RustCrypto` cipher types are distinct types rather than one generic, so
/// the key-size and padding dispatch is expressed once here instead of being
/// written out for each mode and direction.
macro_rules! encrypt_padded {
    ($key:expr, $buffer:expr, $len:expr, $pad:expr,
     $t128:ty, $t192:ty, $t256:ty, $ctor:tt) => {
        match ($key.len(), $pad) {
            (16, true) => <$t128>::new $ctor
                .encrypt_padded_mut::<NoPadding>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            (24, true) => <$t192>::new $ctor
                .encrypt_padded_mut::<NoPadding>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            (32, true) => <$t256>::new $ctor
                .encrypt_padded_mut::<NoPadding>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            (16, false) => <$t128>::new $ctor
                .encrypt_padded_mut::<Pkcs7>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            (24, false) => <$t192>::new $ctor
                .encrypt_padded_mut::<Pkcs7>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            (32, false) => <$t256>::new $ctor
                .encrypt_padded_mut::<Pkcs7>($buffer, $len)
                .map_err(fail_len)?
                .to_vec(),
            _ => return Err(invalid_key()),
        }
    };
}

/// Dispatches a padded decryption over the three AES key sizes.
macro_rules! decrypt_padded {
    ($key:expr, $buffer:expr, $pad:expr,
     $t128:ty, $t192:ty, $t256:ty, $ctor:tt) => {
        match ($key.len(), $pad) {
            (16, true) => <$t128>::new $ctor
                .decrypt_padded_mut::<NoPadding>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            (24, true) => <$t192>::new $ctor
                .decrypt_padded_mut::<NoPadding>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            (32, true) => <$t256>::new $ctor
                .decrypt_padded_mut::<NoPadding>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            (16, false) => <$t128>::new $ctor
                .decrypt_padded_mut::<Pkcs7>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            (24, false) => <$t192>::new $ctor
                .decrypt_padded_mut::<Pkcs7>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            (32, false) => <$t256>::new $ctor
                .decrypt_padded_mut::<Pkcs7>($buffer)
                .map_err(fail_dec)?
                .to_vec(),
            _ => return Err(invalid_key()),
        }
    };
}

/// Reserves room for the padding block PKCS#7 may append.
fn encrypt_buffer(input: &[u8], no_padding: bool) -> Vec<u8> {
    let mut buffer = input.to_vec();
    if !no_padding {
        buffer.resize(input.len() + 16, 0);
    }
    buffer
}

pub(super) fn encrypt_cbc(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = encrypt_buffer(input, no_padding);
    Ok(encrypt_padded!(
        key,
        &mut buffer,
        input.len(),
        no_padding,
        Aes128CbcEnc,
        Aes192CbcEnc,
        Aes256CbcEnc,
        (key.into(), iv.into())
    ))
}

pub(super) fn decrypt_cbc(
    input: &[u8],
    key: &[u8],
    iv: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    Ok(decrypt_padded!(
        key,
        &mut buffer,
        no_padding,
        Aes128CbcDec,
        Aes192CbcDec,
        Aes256CbcDec,
        (key.into(), iv.into())
    ))
}

pub(super) fn encrypt_ecb(
    input: &[u8],
    key: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = encrypt_buffer(input, no_padding);
    Ok(encrypt_padded!(
        key,
        &mut buffer,
        input.len(),
        no_padding,
        Aes128EcbEnc,
        Aes192EcbEnc,
        Aes256EcbEnc,
        (key.into())
    ))
}

pub(super) fn decrypt_ecb(
    input: &[u8],
    key: &[u8],
    no_padding: bool,
) -> Result<Vec<u8>, OperationError> {
    let mut buffer = input.to_vec();
    Ok(decrypt_padded!(
        key,
        &mut buffer,
        no_padding,
        Aes128EcbDec,
        Aes192EcbDec,
        Aes256EcbDec,
        (key.into())
    ))
}
