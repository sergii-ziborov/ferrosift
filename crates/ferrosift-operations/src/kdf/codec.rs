//! PBKDF2 and scrypt key derivation for `CyberChef` 11.3.

use alloc::string::String;
use alloc::vec;

use ferrosift_core::{OperationContext, OperationError};
use hmac::Hmac;
use md5::Md5;
use pbkdf2::pbkdf2;
use scrypt::{Params, scrypt};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const INVALID_HASH: &str = "crypto.pbkdf2.invalid_hash";
const INVALID_PARAMS: &str = "crypto.kdf.invalid_params";
const EMPTY_SALT: &str = "crypto.pbkdf2.empty_salt";
const DERIVE_FAILED: &str = "crypto.kdf.derive_failed";

pub(super) fn pbkdf2_key(
    passphrase: &[u8],
    key_size_bits: i128,
    iterations: i128,
    hash: &str,
    salt: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if salt.is_empty() {
        // CyberChef generates a random salt when empty; FerroSift stays deterministic.
        return Err(failed(EMPTY_SALT));
    }
    let key_len = bits_to_bytes(key_size_bits)?;
    let iters = u32::try_from(iterations).map_err(|_| failed(INVALID_PARAMS))?;
    if iters == 0 || key_len == 0 {
        return Err(failed(INVALID_PARAMS));
    }
    ensure_budget(key_len, context)?;
    // PBKDF2 derives one hash-sized block at a time and each block costs two
    // compressions per iteration, because an HMAC is two hashes. The count
    // comes straight from an argument that will accept four billion, and the
    // answer is sixteen bytes either way -- so this is the only thing between
    // a recipe and an hour of CPU nothing can interrupt.
    let blocks = key_len.div_ceil(digest_bytes(hash)?);
    context.ensure_work(
        u64::try_from(blocks)
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::from(iters))
            .saturating_mul(2),
    )?;
    let mut out = vec![0u8; key_len];
    match hash {
        "SHA1" => pbkdf2::<Hmac<Sha1>>(passphrase, salt, iters, &mut out)
            .map_err(|_| failed(DERIVE_FAILED))?,
        "SHA256" => pbkdf2::<Hmac<Sha256>>(passphrase, salt, iters, &mut out)
            .map_err(|_| failed(DERIVE_FAILED))?,
        "SHA384" => pbkdf2::<Hmac<Sha384>>(passphrase, salt, iters, &mut out)
            .map_err(|_| failed(DERIVE_FAILED))?,
        "SHA512" => pbkdf2::<Hmac<Sha512>>(passphrase, salt, iters, &mut out)
            .map_err(|_| failed(DERIVE_FAILED))?,
        "MD5" => pbkdf2::<Hmac<Md5>>(passphrase, salt, iters, &mut out)
            .map_err(|_| failed(DERIVE_FAILED))?,
        _ => return Err(failed(INVALID_HASH)),
    }
    context.ensure_active()?;
    Ok(to_hex_lower(&out))
}

pub(super) fn scrypt_key(
    password: &[u8],
    salt: &[u8],
    n: i128,
    r: i128,
    p: i128,
    key_length: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let key_len = usize::try_from(key_length).map_err(|_| failed(INVALID_PARAMS))?;
    if key_len == 0 {
        return Err(failed(INVALID_PARAMS));
    }
    ensure_budget(key_len, context)?;
    let log_n = log2_power_of_two(n)?;
    let r_u32 = u32::try_from(r).map_err(|_| failed(INVALID_PARAMS))?;
    let p_u32 = u32::try_from(p).map_err(|_| failed(INVALID_PARAMS))?;
    if r_u32 == 0 || p_u32 == 0 {
        return Err(failed(INVALID_PARAMS));
    }
    // scrypt takes both its memory and its time from arguments, and returns a
    // key whose length says nothing about either. The memory is `128 * r * N`
    // by construction -- that block is the algorithm, not an implementation
    // detail -- and the work is proportional to `N * r * p`.
    let n_units = u64::try_from(n).map_err(|_| failed(INVALID_PARAMS))?;
    let r_units = u64::from(r_u32);
    let p_units = u64::from(p_u32);
    context.ensure_transient(
        n_units
            .saturating_mul(r_units)
            .saturating_mul(128)
            .saturating_add(key_len as u64),
    )?;
    context.ensure_work(n_units.saturating_mul(r_units).saturating_mul(p_units))?;
    let params = Params::new(log_n, r_u32, p_u32, key_len).map_err(|_| failed(INVALID_PARAMS))?;
    let mut out = vec![0u8; key_len];
    scrypt(password, salt, &params, &mut out).map_err(|_| failed(DERIVE_FAILED))?;
    context.ensure_active()?;
    Ok(to_hex_lower(&out))
}

/// How many bytes one block of the named hash produces.
///
/// PBKDF2 asks for `ceil(key_len / digest_len)` blocks, so this is what turns a
/// key length into a block count. Reading it from the name rather than from the
/// instantiated type keeps the estimate on the same side of the `match` as the
/// argument that chose it.
fn digest_bytes(hash: &str) -> Result<usize, OperationError> {
    match hash {
        "MD5" => Ok(16),
        "SHA1" => Ok(20),
        "SHA256" => Ok(32),
        "SHA384" => Ok(48),
        "SHA512" => Ok(64),
        _ => Err(failed(INVALID_HASH)),
    }
}

fn bits_to_bytes(bits: i128) -> Result<usize, OperationError> {
    if bits <= 0 || bits % 8 != 0 {
        return Err(failed(INVALID_PARAMS));
    }
    usize::try_from(bits / 8).map_err(|_| failed(INVALID_PARAMS))
}

fn log2_power_of_two(n: i128) -> Result<u8, OperationError> {
    if n < 2 || (n & (n - 1)) != 0 {
        return Err(failed(INVALID_PARAMS));
    }
    let log = n.trailing_zeros();
    u8::try_from(log).map_err(|_| failed(INVALID_PARAMS))
}

fn ensure_budget(len: usize, context: &OperationContext<'_>) -> Result<(), OperationError> {
    if u64::try_from(len).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
