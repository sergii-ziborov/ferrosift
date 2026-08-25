use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};
use md5::{Digest as _, Md5};
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512, Sha512_224, Sha512_256};
use sha3::{
    Keccak224, Keccak256, Keccak384, Keccak512, Sha3_224, Sha3_256, Sha3_384, Sha3_512, Shake128,
    Shake256,
    digest::{ExtendableOutput, Update as _, XofReader as _},
};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const INVALID_SIZE: &str = "hash.sha2.invalid_size";
const INVALID_SHA3_SIZE: &str = "hash.sha3.invalid_size";
const INVALID_KECCAK_SIZE: &str = "hash.keccak.invalid_size";
const INVALID_SHAKE_CAPACITY: &str = "hash.shake.invalid_capacity";
const INVALID_SHAKE_SIZE: &str = "hash.shake.invalid_size";
const UNSUPPORTED_ROUNDS: &str = "hash.unsupported_rounds";

pub(super) fn md5(input: &[u8], context: &OperationContext<'_>) -> Result<String, OperationError> {
    context.ensure_active()?;
    let digest = Md5::digest(input);
    context.ensure_active()?;
    Ok(to_hex_lower(&digest))
}

pub(super) fn sha1(
    input: &[u8],
    rounds: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    if rounds != 80 {
        return Err(failed(UNSUPPORTED_ROUNDS));
    }
    context.ensure_active()?;
    let digest = Sha1::digest(input);
    context.ensure_active()?;
    Ok(to_hex_lower(&digest))
}

pub(super) fn sha2(
    input: &[u8],
    size: &str,
    rounds_256: i128,
    rounds_512: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let full_256 = rounds_256 == 64;
    let full_512 = rounds_512 == 160;
    context.ensure_active()?;
    let hex = match size {
        "224" if full_256 => to_hex_lower(&Sha224::digest(input)),
        "256" if full_256 => to_hex_lower(&Sha256::digest(input)),
        "384" if full_512 => to_hex_lower(&Sha384::digest(input)),
        "512" if full_512 => to_hex_lower(&Sha512::digest(input)),
        "512/224" if full_512 => to_hex_lower(&Sha512_224::digest(input)),
        "512/256" if full_512 => to_hex_lower(&Sha512_256::digest(input)),
        "224" | "256" if !full_256 => return Err(failed(UNSUPPORTED_ROUNDS)),
        "384" | "512" | "512/224" | "512/256" if !full_512 => {
            return Err(failed(UNSUPPORTED_ROUNDS));
        }
        _ => return Err(failed(INVALID_SIZE)),
    };
    context.ensure_active()?;
    Ok(hex)
}

pub(super) fn sha3(
    input: &[u8],
    size: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let hex = match size {
        "224" => to_hex_lower(&Sha3_224::digest(input)),
        "256" => to_hex_lower(&Sha3_256::digest(input)),
        "384" => to_hex_lower(&Sha3_384::digest(input)),
        "512" => to_hex_lower(&Sha3_512::digest(input)),
        _ => return Err(failed(INVALID_SHA3_SIZE)),
    };
    context.ensure_active()?;
    Ok(hex)
}

/// Keccak as submitted to the SHA-3 competition, which is *not* SHA-3.
///
/// The two differ only in the padding appended before the permutation — SHA-3
/// appends `0x06`, original Keccak `0x01` — so they share a name, a structure,
/// and nothing about their output. Keeping both is not duplication: a digest
/// found in the wild predating FIPS 202 is this one, and reading it with the
/// SHA-3 operation returns a wrong answer rather than an error.
pub(super) fn keccak(
    input: &[u8],
    size: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let hex = match size {
        "224" => to_hex_lower(&Keccak224::digest(input)),
        "256" => to_hex_lower(&Keccak256::digest(input)),
        "384" => to_hex_lower(&Keccak384::digest(input)),
        "512" => to_hex_lower(&Keccak512::digest(input)),
        _ => return Err(failed(INVALID_KECCAK_SIZE)),
    };
    context.ensure_active()?;
    Ok(hex)
}

/// SHAKE, the extendable-output half of the Keccak family.
///
/// `size` is a bit count, following the reference, and the two capacities are
/// named by their security level rather than their rate. A size that is not a
/// whole number of bytes is refused: SHAKE is defined for any bit length, but
/// the output here is hex, and there is no honest way to render half a byte.
/// Refusing says so; truncating would quietly answer a different question.
pub(super) fn shake(
    input: &[u8],
    capacity: &str,
    size: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if size < 0 || size % 8 != 0 {
        return Err(failed(INVALID_SHAKE_SIZE));
    }
    let bytes = usize::try_from(size / 8).map_err(|_| failed(INVALID_SHAKE_SIZE))?;
    let mut output = alloc::vec![0u8; bytes];
    match capacity {
        "128" => {
            let mut hasher = Shake128::default();
            hasher.update(input);
            hasher.finalize_xof().read(&mut output);
        }
        "256" => {
            let mut hasher = Shake256::default();
            hasher.update(input);
            hasher.finalize_xof().read(&mut output);
        }
        _ => return Err(failed(INVALID_SHAKE_CAPACITY)),
    }
    context.ensure_active()?;
    Ok(to_hex_lower(&output))
}
