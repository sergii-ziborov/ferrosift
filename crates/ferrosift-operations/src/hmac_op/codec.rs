use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512, Sha512_224, Sha512_256};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const INVALID_HASH: &str = "hash.hmac.invalid_function";

pub(super) fn hmac(
    input: &[u8],
    key: &[u8],
    function: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let digest = match function {
        "MD5" => finalize::<Hmac<Md5>>(key, input)?,
        "SHA1" => finalize::<Hmac<Sha1>>(key, input)?,
        "SHA224" => finalize::<Hmac<Sha224>>(key, input)?,
        "SHA256" => finalize::<Hmac<Sha256>>(key, input)?,
        "SHA384" => finalize::<Hmac<Sha384>>(key, input)?,
        "SHA512" => finalize::<Hmac<Sha512>>(key, input)?,
        "SHA512/224" => finalize::<Hmac<Sha512_224>>(key, input)?,
        "SHA512/256" => finalize::<Hmac<Sha512_256>>(key, input)?,
        _ => return Err(failed(INVALID_HASH)),
    };
    context.ensure_active()?;
    Ok(to_hex_lower(&digest))
}

fn finalize<M: Mac + hmac::digest::KeyInit>(
    key: &[u8],
    input: &[u8],
) -> Result<alloc::vec::Vec<u8>, OperationError> {
    let mut mac = <M as Mac>::new_from_slice(key).map_err(|_| failed(INVALID_HASH))?;
    mac.update(input);
    Ok(mac.finalize().into_bytes().to_vec())
}
