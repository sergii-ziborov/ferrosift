//! Identity and argument defaults for each classical cipher.
//!
//! These tables are the reference's own names and defaults; keeping them
//! beside the dispatch would push one file past the size the rest of the
//! repository holds to.

use alloc::{vec, vec::Vec};

use ferrosift_model::ArgumentSpec;

use crate::args::{integer_argument, text_argument};

use super::affine;
use super::operation::Kind;

/// Identifier, display name, description, and arguments for one cipher.
pub(super) struct Definition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub arguments: Vec<ArgumentSpec>,
}

/// The reference's own identity and defaults for each cipher.
pub(super) fn definition(kind: &Kind) -> Definition {
    substitution(kind).unwrap_or_else(|| transposition(kind))
}

/// The ciphers that replace one letter with another.
fn substitution(kind: &Kind) -> Option<Definition> {
    let (id, display_name, description, arguments) = match kind {
        Kind::A1z26Encode => (
            "cipher.a1z26.encode@1",
            "A1Z26 Cipher Encode",
            "Replaces each letter with its position in the alphabet.",
            vec![text_argument(
                "delimiter",
                "Field delimiter token.",
                "Space",
            )],
        ),
        Kind::A1z26Decode => (
            "cipher.a1z26.decode@1",
            "A1Z26 Cipher Decode",
            "Reads alphabet positions back into letters.",
            vec![text_argument(
                "delimiter",
                "Field delimiter token.",
                "Space",
            )],
        ),
        Kind::Affine(affine::Direction::Encode) => (
            "cipher.affine.encode@1",
            "Affine Cipher Encode",
            "Applies the affine substitution ax + b modulo 26.",
            affine_arguments(),
        ),
        Kind::Affine(affine::Direction::Decode) => (
            "cipher.affine.decode@1",
            "Affine Cipher Decode",
            "Reverses the affine substitution ax + b modulo 26.",
            affine_arguments(),
        ),
        Kind::Atbash => (
            "cipher.atbash@1",
            "Atbash Cipher",
            "Maps each letter to its mirror image in the alphabet.",
            vec![],
        ),
        Kind::Vigenere(affine::Shift::Forward) => (
            "cipher.vigenere.encode@1",
            "Vigenère Encode",
            "Shifts each letter by the corresponding key letter.",
            vec![text_argument("key", "Key of letters.", "")],
        ),
        Kind::Vigenere(affine::Shift::Backward) => (
            "cipher.vigenere.decode@1",
            "Vigenère Decode",
            "Reverses the Vigenère shift.",
            vec![text_argument("key", "Key of letters.", "")],
        ),
        _ => return None,
    };
    Some(Definition {
        id,
        display_name,
        description,
        arguments,
    })
}

/// The ciphers that rearrange or respell rather than substitute.
fn transposition(kind: &Kind) -> Definition {
    let (id, display_name, description, arguments) = match kind {
        Kind::CaesarBox => (
            "cipher.caesar_box@1",
            "Caesar Box Cipher",
            "Reads the message down the columns of a fixed-height box.",
            vec![integer_argument("box_height", "Box height in rows.", 1)],
        ),
        Kind::CetaceanEncode => (
            "cipher.cetacean.encode@1",
            "Cetacean Cipher Encode",
            "Encodes each code unit as sixteen e/E bits.",
            vec![],
        ),
        Kind::CetaceanDecode => (
            "cipher.cetacean.decode@1",
            "Cetacean Cipher Decode",
            "Reads sixteen-bit e/E groups back into characters.",
            vec![],
        ),
        Kind::Leet => (
            "text.leet@1",
            "Convert Leet Speak",
            "Converts text to or from leet speak.",
            vec![text_argument(
                "direction",
                "To Leet Speak or From Leet Speak.",
                "To Leet Speak",
            )],
        ),
        Kind::Nato => (
            "text.nato@1",
            "Convert to NATO alphabet",
            "Spells each letter, digit, and mark with its NATO word.",
            vec![],
        ),
        Kind::RailFenceEncode => (
            "cipher.rail_fence.encode@1",
            "Rail Fence Cipher Encode",
            "Writes the message along a zig-zag of rails and reads the rails.",
            rail_arguments(),
        ),
        Kind::RailFenceDecode => (
            "cipher.rail_fence.decode@1",
            "Rail Fence Cipher Decode",
            "Rebuilds a message written along a zig-zag of rails.",
            rail_arguments(),
        ),
        _ => (
            "cipher.rot8000@1",
            "ROT8000",
            "Rotates each code unit halfway through the printable BMP.",
            vec![],
        ),
    };
    Definition {
        id,
        display_name,
        description,
        arguments,
    }
}

fn affine_arguments() -> Vec<ArgumentSpec> {
    vec![
        integer_argument("a", "Multiplier, which must be coprime to 26.", 1),
        integer_argument("b", "Additive shift.", 0),
    ]
}

fn rail_arguments() -> Vec<ArgumentSpec> {
    vec![
        integer_argument("key", "Number of rails.", 2),
        integer_argument("offset", "Starting position in the zig-zag.", 0),
    ]
}
