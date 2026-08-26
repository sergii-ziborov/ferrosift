//! LS47, a 7x7 widening of the `ElsieFour` hand cipher.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The 49 characters the cipher can carry, in grid order.
const LETTERS: &[u8; 49] = b"_abcdefghijklmnopqrstuvwxyz.0123456789,-+*/:?!'()";

/// The grid is square, and every coordinate wraps within it.
const SIDE: usize = 7;

/// A position in the 7x7 grid.
type Position = (usize, usize);

/// The key: for each of the 49 grid cells, which letter sits there.
type Key = [u8; 49];

/// One character as the byte the grid stores, if the alphabet has it.
///
/// The key holds the letters themselves rather than their alphabet indices,
/// because rotation moves letters between cells and an index would then mean
/// two different things depending on which grid was being read.
fn to_byte(letter: char) -> Result<u8, OperationError> {
    let byte = u8::try_from(u32::from(letter))
        .map_err(|_| failed("crypto.ls47.letter_not_in_alphabet"))?;
    if LETTERS.contains(&byte) {
        Ok(byte)
    } else {
        Err(failed("crypto.ls47.letter_not_in_alphabet"))
    }
}

/// Where a letter sits in the *alphabet*, which never moves.
fn alphabet_position(letter: u8) -> Position {
    let index = LETTERS
        .iter()
        .position(|candidate| *candidate == letter)
        .unwrap_or(0);
    (index / SIDE, index % SIDE)
}

/// Where a letter sits in the *key*, which moves after every character.
fn key_position(key: &Key, letter: u8) -> Result<Position, OperationError> {
    key.iter()
        .position(|candidate| *candidate == letter)
        .map(|index| (index / SIDE, index % SIDE))
        .ok_or_else(|| failed("crypto.ls47.letter_not_in_key"))
}

/// The letter at a grid position.
fn at(key: &Key, position: Position) -> u8 {
    key[position.1 + position.0 * SIDE]
}

/// Coordinate addition, wrapping at the edge of the grid.
fn add(a: Position, b: Position) -> Position {
    ((a.0 + b.0) % SIDE, (a.1 + b.1) % SIDE)
}

/// Coordinate subtraction, wrapping at the edge of the grid.
///
/// The reference works around JavaScript's remainder returning a negative
/// value for a negative left operand. Adding a full turn before the remainder
/// is that workaround without leaving unsigned arithmetic: both coordinates
/// are already below `SIDE`, so one turn is always enough to stay positive.
fn subtract(a: Position, b: Position) -> Position {
    ((a.0 + SIDE - b.0) % SIDE, (a.1 + SIDE - b.1) % SIDE)
}

/// Rotates one row, moving its contents towards higher columns.
///
/// The distance is inverted first -- `(7 - n % 7) % 7` -- so a rotation of one
/// moves the last cell to the front rather than the first cell to the back.
fn rotate_right(key: &Key, row: usize, distance: usize) -> Key {
    let shift = (SIDE - distance % SIDE) % SIDE;
    let mut rotated = *key;
    for column in 0..SIDE {
        rotated[row * SIDE + column] = key[row * SIDE + (column + shift) % SIDE];
    }
    rotated
}

/// Rotates one column, moving its contents towards higher rows.
fn rotate_down(key: &Key, column: usize, distance: usize) -> Key {
    let shift = (SIDE - distance % SIDE) % SIDE;
    let mut rotated = *key;
    for row in 0..SIDE {
        rotated[row * SIDE + column] = key[((row + shift) % SIDE) * SIDE + column];
    }
    rotated
}

/// Expands a password into a key by rotating the grid once per character.
///
/// Each character rotates one row and one column, and which row is used walks
/// forward with the password rather than being derived from the character --
/// so the same letter twice does not undo itself.
pub(super) fn derive_key(password: &str) -> Result<Key, OperationError> {
    let mut key: Key = *LETTERS;
    let mut row = 0;
    for letter in password.chars() {
        let (letter_row, letter_column) = alphabet_position(to_byte(letter)?);
        key = rotate_right(&key, row, letter_column);
        key = rotate_down(&key, row, letter_row);
        row = (row + 1) % SIDE;
    }
    Ok(key)
}

/// Encrypts, moving the key and the marker after every character.
fn encrypt(key: &Key, plaintext: &str) -> Result<String, OperationError> {
    let mut key = *key;
    let mut marker: Position = (0, 0);
    let mut output = String::with_capacity(plaintext.len());
    for letter in plaintext.chars() {
        let plain = to_byte(letter)?;
        let plain_position = key_position(&key, plain)?;
        let mix = alphabet_position(at(&key, marker));
        let cipher_position = add(plain_position, mix);
        let cipher = at(&key, cipher_position);
        output.push(char::from(cipher));

        key = rotate_right(&key, plain_position.0, 1);
        let moved = key_position(&key, cipher)?;
        key = rotate_down(&key, moved.1, 1);
        marker = add(marker, alphabet_position(cipher));
    }
    Ok(output)
}

/// Decrypts, moving the key and the marker exactly as encryption did.
fn decrypt(key: &Key, ciphertext: &str) -> Result<String, OperationError> {
    let mut key = *key;
    let mut marker: Position = (0, 0);
    let mut output = String::with_capacity(ciphertext.len());
    for letter in ciphertext.chars() {
        let cipher = to_byte(letter)?;
        let cipher_position = key_position(&key, cipher)?;
        let mix = alphabet_position(at(&key, marker));
        let plain_position = subtract(cipher_position, mix);
        output.push(char::from(at(&key, plain_position)));

        key = rotate_right(&key, plain_position.0, 1);
        let moved = key_position(&key, cipher)?;
        key = rotate_down(&key, moved.1, 1);
        marker = add(marker, alphabet_position(cipher));
    }
    Ok(output)
}

/// Encrypts with leading padding and a trailing signature.
///
/// The padding is caller-supplied rather than generated here. The reference
/// draws it from `Math.random`, which makes its output unreproducible -- so
/// the random draw is the operation's business and the cipher stays a
/// function of its inputs.
pub(super) fn encrypt_padded(
    key: &Key,
    plaintext: &str,
    signature: &str,
    padding: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut message = String::with_capacity(padding.len() + plaintext.len() + 3 + signature.len());
    message.push_str(padding);
    message.push_str(plaintext);
    message.push_str("---");
    message.push_str(signature);
    encrypt(key, &message)
}

/// Decrypts and drops the leading padding.
///
/// The count is signed because the reference reaches `String.prototype.slice`
/// with whatever `parseInt` produced. A negative count there counts back from
/// the end and returns a *suffix*, which is a different operation than the one
/// the argument's name suggests; it is reproduced rather than corrected.
pub(super) fn decrypt_padded(
    key: &Key,
    ciphertext: &str,
    padding: i64,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let plain = decrypt(key, ciphertext)?;
    let characters: Vec<char> = plain.chars().collect();
    let length = i64::try_from(characters.len()).unwrap_or(i64::MAX);
    let start = if padding < 0 {
        (length + padding).max(0)
    } else {
        padding.min(length)
    };
    let start = usize::try_from(start).unwrap_or(0);
    Ok(characters[start..].iter().collect())
}
