use alloc::string::String;
use alloc::vec::Vec;

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

/// Streebog at one of its two published digest sizes.
///
/// GOST R 34.11-2012, which the reference reaches through a bundled GOST
/// implementation and this reaches through the published algorithm. The two
/// agree because the algorithm is a standard with one answer -- unlike the
/// libraries this project usually has to reproduce rather than replace, where
/// two correct implementations can differ in whitespace or attribute order and
/// only one of them is the reference's.
///
/// The two sizes are not the same function truncated. Each has its own initial
/// vector, which is why this dispatches rather than trimming a digest.
pub(super) fn streebog(
    input: &[u8],
    size: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let output = match size {
        256 => to_hex_lower(&streebog::Streebog256::digest(input)),
        512 => to_hex_lower(&streebog::Streebog512::digest(input)),
        _ => return Err(failed(UNSUPPORTED_SIZE)),
    };
    context.ensure_active()?;
    Ok(output)
}

/// Which of the two BLAKE2 functions to run.
#[derive(Clone, Copy)]
pub(super) enum Blake2Kind {
    /// The sixty-four bit variant, digests up to sixty-four bytes.
    B,
    /// The thirty-two bit variant, digests up to thirty-two bytes.
    S,
}

/// BLAKE2 at a chosen digest size, optionally keyed.
///
/// The size is in *bits* in the reference's interface and in bytes in the
/// algorithm, so it is divided here rather than at the call site -- a size the
/// function cannot produce is refused rather than rounded to one it can.
///
/// An empty key is not a key. The reference turns a zero-length key into
/// `null` before hashing, and keyed BLAKE2 with an empty key is a different
/// function from unkeyed BLAKE2 -- so passing the empty slice through would
/// answer a digest the reference never produces.
pub(super) fn blake2(
    input: &[u8],
    kind: Blake2Kind,
    size_bits: i128,
    key: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    use blake2::digest::{KeyInit, Mac, Update, VariableOutput};

    context.ensure_active()?;
    if size_bits <= 0 || size_bits % 8 != 0 {
        return Err(failed(UNSUPPORTED_SIZE));
    }
    let size = usize::try_from(size_bits / 8).map_err(|_| failed(UNSUPPORTED_SIZE))?;

    // Keyed and unkeyed BLAKE2 are different functions, not one with an
    // optional extra. The reference turns an empty key into `null` before
    // hashing, so the empty slice takes the unkeyed path here rather than
    // being fed to the keyed one.
    let output = if key.is_empty() {
        let mut buffer = alloc::vec![0_u8; size];
        match kind {
            Blake2Kind::B => {
                let mut hasher =
                    blake2::Blake2bVar::new(size).map_err(|_| failed(UNSUPPORTED_SIZE))?;
                Update::update(&mut hasher, input);
                hasher
                    .finalize_variable(&mut buffer)
                    .map_err(|_| failed(UNSUPPORTED_SIZE))?;
            }
            Blake2Kind::S => {
                let mut hasher =
                    blake2::Blake2sVar::new(size).map_err(|_| failed(UNSUPPORTED_SIZE))?;
                Update::update(&mut hasher, input);
                hasher
                    .finalize_variable(&mut buffer)
                    .map_err(|_| failed(UNSUPPORTED_SIZE))?;
            }
        }
        buffer
    } else {
        // The digest length is part of BLAKE2's parameter block, so a keyed
        // digest of thirty-two bytes is a different computation from the first
        // thirty-two bytes of a keyed sixty-four-byte one. Truncating would
        // have been shorter and wrong; each size is its own instantiation.
        macro_rules! keyed {
            ($mac:ty) => {{
                let mut mac =
                    <$mac as KeyInit>::new_from_slice(key).map_err(|_| failed(UNSUPPORTED_SIZE))?;
                Mac::update(&mut mac, input);
                mac.finalize().into_bytes().to_vec()
            }};
        }
        use blake2::digest::consts::{U16, U20, U32, U48, U64};
        match (kind, size) {
            (Blake2Kind::B, 64) => keyed!(blake2::Blake2bMac<U64>),
            (Blake2Kind::B, 48) => keyed!(blake2::Blake2bMac<U48>),
            (Blake2Kind::B, 32) => keyed!(blake2::Blake2bMac<U32>),
            (Blake2Kind::B, 20) => keyed!(blake2::Blake2bMac<U20>),
            (Blake2Kind::B, 16) => keyed!(blake2::Blake2bMac<U16>),
            (Blake2Kind::S, 32) => keyed!(blake2::Blake2sMac<U32>),
            (Blake2Kind::S, 20) => keyed!(blake2::Blake2sMac<U20>),
            (Blake2Kind::S, 16) => keyed!(blake2::Blake2sMac<U16>),
            _ => return Err(failed(UNSUPPORTED_SIZE)),
        }
    };
    context.ensure_active()?;
    Ok(output)
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
