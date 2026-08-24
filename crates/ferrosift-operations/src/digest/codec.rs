use alloc::string::String;

use digest::Digest;
use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const UNSUPPORTED_ROUNDS: &str = "hash.unsupported_rounds";
const UNSUPPORTED_VARIANT: &str = "hash.unsupported_variant";
const UNSUPPORTED_SIZE: &str = "hash.unsupported_size";

/// Digests whose only parameter is which one to run.
#[derive(Clone, Copy)]
pub(super) enum Simple {
    Md2,
    Md4,
    Sm3,
    Whirlpool,
}

/// Runs one fixed-parameter digest.
///
/// The reference exposes a round count on MD2, SM3, and Whirlpool so that
/// reduced-round variants can be studied. Those are research constructions
/// rather than the published functions, and this implements the published
/// ones — so a non-standard round count is refused rather than answered with
/// a digest from a different algorithm.
pub(super) fn simple(
    input: &[u8],
    which: Simple,
    rounds: Option<i128>,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let standard = match which {
        Simple::Md2 => 18,
        Simple::Md4 => 0,
        Simple::Sm3 => 64,
        Simple::Whirlpool => 10,
    };
    if let Some(rounds) = rounds
        && rounds != standard
    {
        return Err(failed(UNSUPPORTED_ROUNDS));
    }
    let output = match which {
        Simple::Md2 => to_hex_lower(&md2::Md2::digest(input)),
        Simple::Md4 => to_hex_lower(&md4::Md4::digest(input)),
        Simple::Sm3 => to_hex_lower(&sm3::Sm3::digest(input)),
        Simple::Whirlpool => to_hex_lower(&whirlpool::Whirlpool::digest(input)),
    };
    context.ensure_active()?;
    Ok(output)
}

/// Checks the Whirlpool variant, which selects a different S-box or round
/// constant schedule rather than a parameter of the same function.
pub(super) fn check_whirlpool_variant(variant: &str) -> Result<(), OperationError> {
    if variant.eq_ignore_ascii_case("Whirlpool") {
        Ok(())
    } else {
        Err(failed(UNSUPPORTED_VARIANT))
    }
}

/// RIPEMD at one of its four published digest sizes.
pub(super) fn ripemd(
    input: &[u8],
    size: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let output = match size {
        128 => to_hex_lower(&ripemd::Ripemd128::digest(input)),
        160 => to_hex_lower(&ripemd::Ripemd160::digest(input)),
        256 => to_hex_lower(&ripemd::Ripemd256::digest(input)),
        320 => to_hex_lower(&ripemd::Ripemd320::digest(input)),
        _ => return Err(failed(UNSUPPORTED_SIZE)),
    };
    context.ensure_active()?;
    Ok(output)
}
