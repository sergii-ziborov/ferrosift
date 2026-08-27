//! TEA and XTEA, with the five block modes the reference wraps them in.
//!
//! Two Feistel ciphers of about ten lines each, and about two hundred lines of
//! everything around them — which is the honest ratio. The block functions are
//! published and have test vectors; the modes, the padding, and what happens to
//! a partial trailing block are the reference's own arrangement and are where a
//! port actually differs.

use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The golden-ratio constant both ciphers add each cycle.
const DELTA: u32 = 0x9E37_79B9;

/// Sixty-four bits, in bytes.
pub(super) const BLOCK: usize = 8;

/// TEA's cycle count, which is not adjustable.
const TEA_CYCLES: u32 = 32;

const INVALID_MODE: &str = "crypto.tea.invalid_mode";
const INVALID_PADDING: &str = "crypto.tea.invalid_padding";
const UNPADDED_INPUT: &str = "crypto.tea.unpadded_input";
const BAD_PADDING: &str = "crypto.tea.bad_padding";
const BAD_LENGTH: &str = "crypto.tea.bad_length";
const RANDOM_PADDING: &str = "crypto.tea.random_padding";

/// Which of the two ciphers, and for XTEA how many cycles.
#[derive(Clone, Copy)]
pub(super) enum Variant {
    /// TEA, always thirty-two cycles.
    Tea,
    /// XTEA, whose cycle count the reference exposes as an argument.
    Xtea(u32),
}

/// The block cipher mode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ecb,
    Cbc,
    Cfb,
    Ofb,
    Ctr,
}

impl Mode {
    fn parse(name: &str) -> Result<Self, OperationError> {
        match name {
            "ECB" => Ok(Self::Ecb),
            "CBC" => Ok(Self::Cbc),
            "CFB" => Ok(Self::Cfb),
            "OFB" => Ok(Self::Ofb),
            "CTR" => Ok(Self::Ctr),
            _ => Err(failed(INVALID_MODE)),
        }
    }

    /// Whether this mode pads and works on whole blocks.
    ///
    /// The other three are stream constructions: they keep the message length
    /// and cut the keystream to it, so nothing is padded and nothing is
    /// stripped.
    const fn is_blocked(self) -> bool {
        matches!(self, Self::Ecb | Self::Cbc)
    }
}

/// The padding scheme, which only the blocked modes use.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Padding {
    Pkcs5,
    No,
    Zero,
    Random,
    Bit,
}

impl Padding {
    fn parse(name: &str) -> Result<Self, OperationError> {
        match name {
            "PKCS5" => Ok(Self::Pkcs5),
            "NO" => Ok(Self::No),
            "ZERO" => Ok(Self::Zero),
            "RANDOM" => Ok(Self::Random),
            "BIT" => Ok(Self::Bit),
            _ => Err(failed(INVALID_PADDING)),
        }
    }
}

/// Four big-endian words from sixteen bytes.
fn key_words(key: &[u8]) -> [u32; 4] {
    let mut words = [0_u32; 4];
    for (index, word) in words.iter_mut().enumerate() {
        let at = index * 4;
        *word = u32::from_be_bytes([key[at], key[at + 1], key[at + 2], key[at + 3]]);
    }
    words
}

/// The two halves of a block, big-endian.
fn block_words(block: &[u8]) -> (u32, u32) {
    (
        u32::from_be_bytes([block[0], block[1], block[2], block[3]]),
        u32::from_be_bytes([block[4], block[5], block[6], block[7]]),
    )
}

/// The two halves back into a block.
fn block_bytes(left: u32, right: u32) -> [u8; BLOCK] {
    let mut block = [0_u8; BLOCK];
    block[..4].copy_from_slice(&left.to_be_bytes());
    block[4..].copy_from_slice(&right.to_be_bytes());
    block
}

impl Variant {
    /// How many cycles this instance runs.
    const fn cycles(self) -> u32 {
        match self {
            Self::Tea => TEA_CYCLES,
            Self::Xtea(cycles) => cycles,
        }
    }

    /// One block, forwards.
    ///
    /// Every intermediate is a wrapping thirty-two bit value. The reference
    /// writes `>>> 0` after each assignment and lets the terms in between be
    /// JavaScript doubles, which is the same thing: `^` and `<<` coerce with
    /// `ToInt32`, so the bits that survive are the low thirty-two either way.
    fn encrypt_block(self, block: &[u8], key: &[u32; 4]) -> [u8; BLOCK] {
        let (mut left, mut right) = block_words(block);
        let mut sum: u32 = 0;
        for _ in 0..self.cycles() {
            match self {
                Self::Tea => {
                    sum = sum.wrapping_add(DELTA);
                    left = left.wrapping_add(
                        ((right << 4).wrapping_add(key[0]))
                            ^ right.wrapping_add(sum)
                            ^ ((right >> 5).wrapping_add(key[1])),
                    );
                    right = right.wrapping_add(
                        ((left << 4).wrapping_add(key[2]))
                            ^ left.wrapping_add(sum)
                            ^ ((left >> 5).wrapping_add(key[3])),
                    );
                }
                Self::Xtea(_) => {
                    left = left.wrapping_add(
                        (((right << 4) ^ (right >> 5)).wrapping_add(right))
                            ^ sum.wrapping_add(key[(sum & 3) as usize]),
                    );
                    sum = sum.wrapping_add(DELTA);
                    right = right.wrapping_add(
                        (((left << 4) ^ (left >> 5)).wrapping_add(left))
                            ^ sum.wrapping_add(key[((sum >> 11) & 3) as usize]),
                    );
                }
            }
        }
        block_bytes(left, right)
    }

    /// One block, backwards.
    fn decrypt_block(self, block: &[u8], key: &[u32; 4]) -> [u8; BLOCK] {
        let (mut left, mut right) = block_words(block);
        let mut sum = DELTA.wrapping_mul(self.cycles());
        for _ in 0..self.cycles() {
            match self {
                Self::Tea => {
                    right = right.wrapping_sub(
                        ((left << 4).wrapping_add(key[2]))
                            ^ left.wrapping_add(sum)
                            ^ ((left >> 5).wrapping_add(key[3])),
                    );
                    left = left.wrapping_sub(
                        ((right << 4).wrapping_add(key[0]))
                            ^ right.wrapping_add(sum)
                            ^ ((right >> 5).wrapping_add(key[1])),
                    );
                    sum = sum.wrapping_sub(DELTA);
                }
                Self::Xtea(_) => {
                    right = right.wrapping_sub(
                        (((left << 4) ^ (left >> 5)).wrapping_add(left))
                            ^ sum.wrapping_add(key[((sum >> 11) & 3) as usize]),
                    );
                    sum = sum.wrapping_sub(DELTA);
                    left = left.wrapping_sub(
                        (((right << 4) ^ (right >> 5)).wrapping_add(right))
                            ^ sum.wrapping_add(key[(sum & 3) as usize]),
                    );
                }
            }
        }
        block_bytes(left, right)
    }
}

/// Everything the two directions share, resolved once.
struct Setup {
    variant: Variant,
    key: [u32; 4],
    iv: Vec<u8>,
    mode: Mode,
    padding: Padding,
}

impl Setup {
    fn new(
        variant: Variant,
        key: &[u8],
        iv: &[u8],
        mode: &str,
        padding: &str,
    ) -> Result<Self, OperationError> {
        Ok(Self {
            variant,
            key: key_words(key),
            // An empty IV is eight null bytes, which is what the reference
            // substitutes rather than refusing.
            iv: if iv.is_empty() {
                alloc::vec![0_u8; BLOCK]
            } else {
                iv.to_vec()
            },
            mode: Mode::parse(mode)?,
            padding: Padding::parse(padding)?,
        })
    }
}

/// Grows the message to a whole number of blocks.
///
/// `RANDOM` is the one scheme with no answer here. The reference fills those
/// bytes with `Math.random()`, so there is no output to be byte-exact against —
/// and this refuses in exactly the cases where the reference would have been
/// unpredictable, which is when padding is actually added. A message that is
/// already a whole number of blocks takes the early return above and never
/// reaches the fill, so `RANDOM` works there just as it does in the reference.
fn pad(message: &[u8], padding: Padding) -> Result<Vec<u8>, OperationError> {
    let remainder = message.len() % BLOCK;
    if remainder == 0 && padding != Padding::Pkcs5 {
        return Ok(message.to_vec());
    }
    let count = if remainder == 0 {
        BLOCK
    } else {
        BLOCK - remainder
    };
    let mut padded = message.to_vec();
    match padding {
        Padding::No => return Err(failed(UNPADDED_INPUT)),
        Padding::Random => return Err(failed(RANDOM_PADDING)),
        Padding::Pkcs5 => padded.extend(core::iter::repeat_n(
            u8::try_from(count).unwrap_or(0),
            count,
        )),
        Padding::Zero => padded.extend(core::iter::repeat_n(0_u8, count)),
        Padding::Bit => {
            padded.push(0x80);
            padded.extend(core::iter::repeat_n(0_u8, count - 1));
        }
    }
    Ok(padded)
}

/// Shrinks the message back, for the two schemes that mark where they stopped.
///
/// `ZERO` and `RANDOM` are *not* stripped. They have no marker to find, so the
/// reference hands back the padded plaintext and this does too — a decrypt with
/// either scheme returns whatever the padding added along with the message.
fn unpad(message: Vec<u8>, padding: Padding) -> Result<Vec<u8>, OperationError> {
    if message.is_empty() {
        return Ok(message);
    }
    match padding {
        Padding::No | Padding::Zero | Padding::Random => Ok(message),
        Padding::Pkcs5 => {
            let count = usize::from(message[message.len() - 1]);
            if count == 0 || count > BLOCK || count > message.len() {
                return Err(failed(BAD_PADDING));
            }
            if message[message.len() - count..]
                .iter()
                .any(|byte| usize::from(*byte) != count)
            {
                return Err(failed(BAD_PADDING));
            }
            Ok(message[..message.len() - count].to_vec())
        }
        Padding::Bit => {
            for at in (0..message.len()).rev() {
                if message[at] == 0x80 {
                    return Ok(message[..at].to_vec());
                }
                if message[at] != 0 {
                    return Err(failed(BAD_PADDING));
                }
            }
            Err(failed(BAD_PADDING))
        }
    }
}

/// One block of the message, zero-filled when the message runs out.
///
/// The three stream modes read a whole block even at the end and then cut the
/// output back to the message length, so the tail is padded here and discarded
/// by the caller.
fn block_at(data: &[u8], at: usize) -> [u8; BLOCK] {
    let mut block = [0_u8; BLOCK];
    let end = (at + BLOCK).min(data.len());
    block[..end - at].copy_from_slice(&data[at..end]);
    block
}

/// XOR of two blocks.
fn xor(left: [u8; BLOCK], right: [u8; BLOCK]) -> [u8; BLOCK] {
    let mut output = [0_u8; BLOCK];
    for (at, byte) in output.iter_mut().enumerate() {
        *byte = left[at] ^ right[at];
    }
    output
}

/// The counter, incremented as a big-endian number.
fn increment(counter: &mut [u8; BLOCK]) {
    for byte in counter.iter_mut().rev() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// The IV as a block, padded or cut to eight bytes.
///
/// The operation has already refused any length but zero and eight outside ECB;
/// inside ECB the reference accepts an IV of any length and never looks at it.
fn iv_block(iv: &[u8]) -> [u8; BLOCK] {
    block_at(iv, 0)
}

/// Encrypts, in whichever mode was asked for.
pub(super) fn encrypt(
    message: &[u8],
    variant: Variant,
    key: &[u8],
    iv: &[u8],
    mode: &str,
    padding: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let setup = Setup::new(variant, key, iv, mode, padding)?;
    // Before the padding, so an empty message is empty output even under a
    // scheme that would otherwise refuse it.
    if message.is_empty() {
        return Ok(Vec::new());
    }
    let length = message.len();
    let data = if setup.mode.is_blocked() {
        pad(message, setup.padding)?
    } else {
        message.to_vec()
    };

    let mut output = Vec::with_capacity(data.len());
    let mut chain = iv_block(&setup.iv);
    let mut at = 0;
    while at < data.len() {
        context.ensure_active()?;
        let block = block_at(&data, at);
        match setup.mode {
            Mode::Ecb => output.extend_from_slice(&setup.variant.encrypt_block(&block, &setup.key)),
            Mode::Cbc => {
                chain = setup.variant.encrypt_block(&xor(block, chain), &setup.key);
                output.extend_from_slice(&chain);
            }
            Mode::Cfb => {
                chain = xor(setup.variant.encrypt_block(&chain, &setup.key), block);
                output.extend_from_slice(&chain);
            }
            Mode::Ofb => {
                chain = setup.variant.encrypt_block(&chain, &setup.key);
                output.extend_from_slice(&xor(chain, block));
            }
            Mode::Ctr => {
                let keystream = setup.variant.encrypt_block(&chain, &setup.key);
                output.extend_from_slice(&xor(keystream, block));
                increment(&mut chain);
            }
        }
        at += BLOCK;
    }

    if setup.mode.is_blocked() {
        Ok(output)
    } else {
        output.truncate(length);
        Ok(output)
    }
}

/// Decrypts, in whichever mode was asked for.
pub(super) fn decrypt(
    ciphertext: &[u8],
    variant: Variant,
    key: &[u8],
    iv: &[u8],
    mode: &str,
    padding: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    let setup = Setup::new(variant, key, iv, mode, padding)?;
    if ciphertext.is_empty() {
        return Ok(Vec::new());
    }
    let length = ciphertext.len();
    if setup.mode.is_blocked() && !length.is_multiple_of(BLOCK) {
        return Err(failed(BAD_LENGTH));
    }

    let mut output = Vec::with_capacity(length);
    let mut chain = iv_block(&setup.iv);
    let mut at = 0;
    while at < length {
        context.ensure_active()?;
        let block = block_at(ciphertext, at);
        match setup.mode {
            Mode::Ecb => output.extend_from_slice(&setup.variant.decrypt_block(&block, &setup.key)),
            Mode::Cbc => {
                let plain = setup.variant.decrypt_block(&block, &setup.key);
                output.extend_from_slice(&xor(plain, chain));
                chain = block;
            }
            Mode::Cfb => {
                // The *encrypting* direction of the cipher, in all three stream
                // modes: what they encrypt is the chain, never the message.
                let keystream = setup.variant.encrypt_block(&chain, &setup.key);
                output.extend_from_slice(&xor(keystream, block));
                chain = block;
            }
            Mode::Ofb => {
                chain = setup.variant.encrypt_block(&chain, &setup.key);
                output.extend_from_slice(&xor(chain, block));
            }
            Mode::Ctr => {
                let keystream = setup.variant.encrypt_block(&chain, &setup.key);
                output.extend_from_slice(&xor(keystream, block));
                increment(&mut chain);
            }
        }
        at += BLOCK;
    }

    if setup.mode.is_blocked() {
        unpad(output, setup.padding)
    } else {
        output.truncate(length);
        Ok(output)
    }
}
