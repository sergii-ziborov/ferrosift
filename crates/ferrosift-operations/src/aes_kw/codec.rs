//! AES Key Wrap / Unwrap (RFC 3394), matching `CyberChef` 11.3.

use alloc::vec::Vec;

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256, Block};
use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID_KEK: &str = "crypto.aes_kw.invalid_kek";
const INVALID_IV: &str = "crypto.aes_kw.invalid_iv";
const INVALID_INPUT: &str = "crypto.aes_kw.invalid_input";
const IV_MISMATCH: &str = "crypto.aes_kw.iv_mismatch";

pub(super) fn wrap(
    input: &[u8],
    kek: &[u8],
    iv: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    validate_kek(kek)?;
    if iv.len() != 8 {
        return Err(failed(INVALID_IV));
    }
    if input.len() < 16 || !input.len().is_multiple_of(8) {
        return Err(failed(INVALID_INPUT));
    }

    let n = input.len() / 8;
    let mut a = [0u8; 8];
    a.copy_from_slice(iv);
    let mut r = Vec::with_capacity(n);
    for chunk in input.chunks_exact(8) {
        let mut block = [0u8; 8];
        block.copy_from_slice(chunk);
        r.push(block);
    }

    let mut counter: u64 = 1;
    for _ in 0..6 {
        for item in &mut r {
            context.ensure_active()?;
            let mut block = [0u8; 16];
            block[..8].copy_from_slice(&a);
            block[8..].copy_from_slice(item);
            aes_encrypt_block(kek, &mut block)?;
            a.copy_from_slice(&block[..8]);
            xor_u64_be(&mut a, counter);
            item.copy_from_slice(&block[8..]);
            counter = counter.wrapping_add(1);
        }
    }

    let mut out = Vec::with_capacity(8 + input.len());
    out.extend_from_slice(&a);
    for item in &r {
        out.extend_from_slice(item);
    }
    ensure_budget(out.len(), context)?;
    context.ensure_active()?;
    Ok(out)
}

pub(super) fn unwrap(
    input: &[u8],
    kek: &[u8],
    iv: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    validate_kek(kek)?;
    if iv.len() != 8 {
        return Err(failed(INVALID_IV));
    }
    if input.len() < 24 || !input.len().is_multiple_of(8) {
        return Err(failed(INVALID_INPUT));
    }

    let n = (input.len() / 8) - 1;
    let mut a = [0u8; 8];
    a.copy_from_slice(&input[..8]);
    let mut r = Vec::with_capacity(n);
    for chunk in input[8..].chunks_exact(8) {
        let mut block = [0u8; 8];
        block.copy_from_slice(chunk);
        r.push(block);
    }

    let mut counter = u64::try_from(n)
        .map_err(|_| failed(INVALID_INPUT))?
        .checked_mul(6)
        .ok_or_else(|| failed(INVALID_INPUT))?;
    for _ in 0..6 {
        for item in r.iter_mut().rev() {
            context.ensure_active()?;
            xor_u64_be(&mut a, counter);
            let mut block = [0u8; 16];
            block[..8].copy_from_slice(&a);
            block[8..].copy_from_slice(item);
            aes_decrypt_block(kek, &mut block)?;
            a.copy_from_slice(&block[..8]);
            item.copy_from_slice(&block[8..]);
            counter = counter.wrapping_sub(1);
        }
    }

    if a.as_slice() != iv {
        return Err(failed(IV_MISMATCH));
    }

    let mut out = Vec::with_capacity(n * 8);
    for item in &r {
        out.extend_from_slice(item);
    }
    ensure_budget(out.len(), context)?;
    context.ensure_active()?;
    Ok(out)
}

fn validate_kek(kek: &[u8]) -> Result<(), OperationError> {
    if matches!(kek.len(), 16 | 24 | 32) {
        Ok(())
    } else {
        Err(failed(INVALID_KEK))
    }
}

fn xor_u64_be(block: &mut [u8; 8], value: u64) {
    let bytes = value.to_be_bytes();
    for (dst, src) in block.iter_mut().zip(bytes.iter()) {
        *dst ^= *src;
    }
}

fn aes_encrypt_block(key: &[u8], block: &mut [u8; 16]) -> Result<(), OperationError> {
    let mut aes_block = Block::from(*block);
    match key.len() {
        16 => Aes128::new(key.into()).encrypt_block(&mut aes_block),
        24 => Aes192::new(key.into()).encrypt_block(&mut aes_block),
        32 => Aes256::new(key.into()).encrypt_block(&mut aes_block),
        _ => return Err(failed(INVALID_KEK)),
    }
    block.copy_from_slice(aes_block.as_slice());
    Ok(())
}

fn aes_decrypt_block(key: &[u8], block: &mut [u8; 16]) -> Result<(), OperationError> {
    let mut aes_block = Block::from(*block);
    match key.len() {
        16 => Aes128::new(key.into()).decrypt_block(&mut aes_block),
        24 => Aes192::new(key.into()).decrypt_block(&mut aes_block),
        32 => Aes256::new(key.into()).decrypt_block(&mut aes_block),
        _ => return Err(failed(INVALID_KEK)),
    }
    block.copy_from_slice(aes_block.as_slice());
    Ok(())
}

fn ensure_budget(len: usize, context: &OperationContext<'_>) -> Result<(), OperationError> {
    if u64::try_from(len).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
