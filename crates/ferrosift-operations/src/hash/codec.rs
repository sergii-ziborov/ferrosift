use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};
use md5::{Digest as _, Md5};
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512, Sha512_224, Sha512_256};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const INVALID_SIZE: &str = "hash.sha2.invalid_size";
const INVALID_SHA3_SIZE: &str = "hash.sha3.invalid_size";
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
