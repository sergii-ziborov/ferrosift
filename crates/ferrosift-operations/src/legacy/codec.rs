use alloc::string::{String, ToString};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::hex_util::to_hex_lower;

const UNSUPPORTED_ROUNDS: &str = "hash.sha0.unsupported_rounds";
const INVALID_SEED: &str = "hash.murmur3.invalid_seed";

/// `MurmurHash3`, 32-bit, as a decimal string.
///
/// The reference emulates 32-bit multiplication by splitting each operand into
/// halves, because JavaScript numbers lose the low bits of a product above
/// 2^53. The split is arithmetically equal to multiplying modulo 2^32, so this
/// wraps instead — same result, and the intent survives.
///
/// Bytes come from `charCodeAt(i) & 0xff`, the low half of each UTF-16 code
/// unit, and the length mixed in at the end is the code-unit count. Both are
/// properties of the string rather than of any encoding of it, so this walks
/// code units to match.
pub(super) fn murmur3(
    input: &str,
    seed: i128,
    signed: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    const C1: u32 = 0xcc9e_2d51;
    const C2: u32 = 0x1b87_3593;

    context.ensure_active()?;
    let seed = u32::try_from(seed).map_err(|_| failed(INVALID_SEED))?;

    let units: alloc::vec::Vec<u8> = input
        .encode_utf16()
        .map(|unit| u8::try_from(unit & 0xff).unwrap_or(0))
        .collect();
    let remainder = units.len() & 3;
    let body = units.len() - remainder;

    let mut h1 = seed;
    let mut index = 0;
    while index < body {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let mut k1 = u32::from(units[index])
            | (u32::from(units[index + 1]) << 8)
            | (u32::from(units[index + 2]) << 16)
            | (u32::from(units[index + 3]) << 24);
        index += 4;

        k1 = k1.wrapping_mul(C1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(C2);

        h1 ^= k1;
        h1 = h1.rotate_left(13);
        h1 = h1.wrapping_mul(5).wrapping_add(0xe654_6b64);
    }

    let mut k1: u32 = 0;
    if remainder == 3 {
        k1 ^= u32::from(units[index + 2]) << 16;
    }
    if remainder >= 2 {
        k1 ^= u32::from(units[index + 1]) << 8;
    }
    if remainder >= 1 {
        k1 ^= u32::from(units[index]);
        k1 = k1.wrapping_mul(C1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(C2);
        h1 ^= k1;
    }

    // The length is mixed in as a 32-bit value; a string longer than 4 GiB of
    // code units would wrap in the reference too, so wrapping is the match.
    h1 ^= u32::try_from(units.len() & 0xffff_ffff).unwrap_or(0);
    h1 ^= h1 >> 16;
    h1 = h1.wrapping_mul(0x85eb_ca6b);
    h1 ^= h1 >> 13;
    h1 = h1.wrapping_mul(0xc2b2_ae35);
    h1 ^= h1 >> 16;

    context.ensure_active()?;
    Ok(if signed {
        h1.cast_signed().to_string()
    } else {
        h1.to_string()
    })
}

/// SHA-0: the withdrawn 1993 original, as lower-case hex.
///
/// It differs from SHA-1 in one place — the message schedule does not rotate
/// the expanded word left by one — and that single omission is the "significant
/// flaw" it was withdrawn for. Nothing else about the two differs, which is why
/// a digest labelled SHA that predates 1995 will not verify against SHA-1.
///
/// The reference exposes a round count so reduced-round variants can be
/// studied. Those are research constructions rather than the published
/// function, so a count other than 80 is refused here rather than answered
/// with a digest from a different algorithm — the same stance the other
/// digests in this crate take.
pub(super) fn sha0(
    input: &[u8],
    rounds: i128,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if rounds != 80 {
        return Err(failed(UNSUPPORTED_ROUNDS));
    }

    let mut state: [u32; 5] = [
        0x6745_2301,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];

    // Merkle-Damgård padding: a one bit, zeros, then the length in bits as a
    // big-endian 64-bit count, landing the whole message on a block boundary.
    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    for (index, block) in message.chunks_exact(64).enumerate() {
        if index.is_multiple_of(64) {
            context.ensure_active()?;
        }
        compress(&mut state, block);
    }

    let mut digest = [0u8; 20];
    for (slot, word) in digest.chunks_exact_mut(4).zip(state) {
        slot.copy_from_slice(&word.to_be_bytes());
    }
    context.ensure_active()?;
    Ok(to_hex_lower(&digest))
}

/// One SHA-0 compression round over a 64-byte block.
fn compress(state: &mut [u32; 5], block: &[u8]) {
    let mut schedule = [0u32; 80];
    for (index, word) in block.chunks_exact(4).enumerate() {
        schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
    }
    for index in 16..80 {
        // SHA-1 rotates this left by one. SHA-0 does not, and that is the
        // whole difference between them.
        schedule[index] =
            schedule[index - 3] ^ schedule[index - 8] ^ schedule[index - 14] ^ schedule[index - 16];
    }

    // The five working variables carry the names the specification gives them.
    // Renaming them would make the round below harder to check against it, not
    // easier to read.
    let [mut var_a, mut var_b, mut var_c, mut var_d, mut var_e] = *state;
    for (index, word) in schedule.iter().enumerate() {
        let (mixed, constant) = match index {
            0..=19 => ((var_b & var_c) | (!var_b & var_d), 0x5a82_7999),
            20..=39 => (var_b ^ var_c ^ var_d, 0x6ed9_eba1),
            40..=59 => (
                (var_b & var_c) | (var_b & var_d) | (var_c & var_d),
                0x8f1b_bcdc,
            ),
            _ => (var_b ^ var_c ^ var_d, 0xca62_c1d6),
        };
        let temp = var_a
            .rotate_left(5)
            .wrapping_add(mixed)
            .wrapping_add(var_e)
            .wrapping_add(constant)
            .wrapping_add(*word);
        var_e = var_d;
        var_d = var_c;
        var_c = var_b.rotate_left(30);
        var_b = var_a;
        var_a = temp;
    }

    state[0] = state[0].wrapping_add(var_a);
    state[1] = state[1].wrapping_add(var_b);
    state[2] = state[2].wrapping_add(var_c);
    state[3] = state[3].wrapping_add(var_d);
    state[4] = state[4].wrapping_add(var_e);
}
