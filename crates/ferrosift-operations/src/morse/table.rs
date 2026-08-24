//! The Morse table, written as the reference writes it.
//!
//! `1` is a dash and `0` a dot, so each entry is the signal pattern without
//! committing to how it is rendered — the operation substitutes the chosen
//! dash and dot afterwards.

/// Character to signal pattern, in the reference's own order. Reversing this
/// table for decoding keeps the last entry when two characters share a signal,
/// exactly as rebuilding the object does.
pub(super) const TABLE: [(char, &str); 45] = [
    ('A', "01"),
    ('B', "1000"),
    ('C', "1010"),
    ('D', "100"),
    ('E', "0"),
    ('F', "0010"),
    ('G', "110"),
    ('H', "0000"),
    ('I', "00"),
    ('J', "0111"),
    ('K', "101"),
    ('L', "0100"),
    ('M', "11"),
    ('N', "10"),
    ('O', "111"),
    ('P', "0110"),
    ('Q', "1101"),
    ('R', "010"),
    ('S', "000"),
    ('T', "1"),
    ('U', "001"),
    ('V', "0001"),
    ('W', "011"),
    ('X', "1001"),
    ('Y', "1011"),
    ('Z', "1100"),
    ('1', "01111"),
    ('2', "00111"),
    ('3', "00011"),
    ('4', "00001"),
    ('5', "00000"),
    ('6', "10000"),
    ('7', "11000"),
    ('8', "11100"),
    ('9', "11110"),
    ('0', "11111"),
    ('.', "010101"),
    (',', "110011"),
    (':', "111000"),
    (';', "101010"),
    ('!', "101011"),
    ('?', "001100"),
    ('\'', "011110"),
    ('"', "010010"),
    ('/', "10010"),
];

/// The entries the reference lists after `/`, kept separate only so neither
/// array reaches the size limit the repository holds to.
pub(super) const TABLE_TAIL: [(char, &str); 9] = [
    ('-', "100001"),
    ('+', "01010"),
    ('(', "10110"),
    (')', "101101"),
    ('@', "011010"),
    ('=', "10001"),
    ('&', "01000"),
    ('_', "001101"),
    ('$', "0001001"),
];

/// A space has its own seven-dot signal, listed last in the reference so that
/// it wins the reverse lookup if anything collides with it.
pub(super) const SPACE: (char, &str) = (' ', "0000000");
