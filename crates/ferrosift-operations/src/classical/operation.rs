//! Operation wrappers for the classical ciphers.

use alloc::{string::String, vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueKind};

use crate::args::{integer_argument, integer_value, text_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{bytes, take_bytes, take_text, text};

use super::catalog::definition;
use super::{a1z26, affine, rot, transpose, whimsy};

/// What a wrapped cipher does with its text.
pub(super) enum Kind {
    A1z26Encode,
    A1z26Decode,
    Affine(affine::Direction),
    Atbash,
    CaesarBox,
    CetaceanEncode,
    CetaceanDecode,
    Leet,
    Nato,
    RailFenceEncode,
    RailFenceDecode,
    Rot8000,
    Vigenere(affine::Shift),
}

/// A text-in, text-out classical cipher.
///
/// These share one wrapper because they differ only in which codec they call
/// and which arguments they read; giving each its own file would be thirteen
/// copies of the same twenty lines.
pub struct ClassicalCipher {
    spec: OperationSpec,
    kind: Kind,
}

impl ClassicalCipher {
    fn build(kind: Kind) -> Self {
        let definition = definition(&kind);
        Self {
            spec: build_uniform(
                ValueKind::Text,
                UniformSpec {
                    id: definition.id,
                    display_name: definition.display_name,
                    category: "Ciphers",
                    description: definition.description,
                    // Every one of these carries the reference's own name.
                    cyberchef_alias: definition.display_name,
                    arguments: definition.arguments,
                },
            ),
            kind,
        }
    }

    /// A1Z26 encode.
    #[must_use]
    pub fn a1z26_encode() -> Self {
        Self::build(Kind::A1z26Encode)
    }

    /// A1Z26 decode.
    #[must_use]
    pub fn a1z26_decode() -> Self {
        Self::build(Kind::A1z26Decode)
    }

    /// Affine encode.
    #[must_use]
    pub fn affine_encode() -> Self {
        Self::build(Kind::Affine(affine::Direction::Encode))
    }

    /// Affine decode.
    #[must_use]
    pub fn affine_decode() -> Self {
        Self::build(Kind::Affine(affine::Direction::Decode))
    }

    /// Atbash.
    #[must_use]
    pub fn atbash() -> Self {
        Self::build(Kind::Atbash)
    }

    /// Caesar Box.
    #[must_use]
    pub fn caesar_box() -> Self {
        Self::build(Kind::CaesarBox)
    }

    /// Cetacean encode.
    #[must_use]
    pub fn cetacean_encode() -> Self {
        Self::build(Kind::CetaceanEncode)
    }

    /// Cetacean decode.
    #[must_use]
    pub fn cetacean_decode() -> Self {
        Self::build(Kind::CetaceanDecode)
    }

    /// Leet speak, in either direction.
    #[must_use]
    pub fn leet() -> Self {
        Self::build(Kind::Leet)
    }

    /// NATO spelling alphabet.
    #[must_use]
    pub fn nato() -> Self {
        Self::build(Kind::Nato)
    }

    /// Rail Fence encode.
    #[must_use]
    pub fn rail_fence_encode() -> Self {
        Self::build(Kind::RailFenceEncode)
    }

    /// Rail Fence decode.
    #[must_use]
    pub fn rail_fence_decode() -> Self {
        Self::build(Kind::RailFenceDecode)
    }

    /// ROT8000.
    #[must_use]
    pub fn rot8000() -> Self {
        Self::build(Kind::Rot8000)
    }

    /// Vigenère encode.
    #[must_use]
    pub fn vigenere_encode() -> Self {
        Self::build(Kind::Vigenere(affine::Shift::Forward))
    }

    /// Vigenère decode.
    #[must_use]
    pub fn vigenere_decode() -> Self {
        Self::build(Kind::Vigenere(affine::Shift::Backward))
    }

    fn apply(
        &self,
        input: &str,
        arguments: &Arguments,
        context: &OperationContext<'_>,
    ) -> Result<String, OperationError> {
        match &self.kind {
            Kind::A1z26Encode => {
                a1z26::a1z26_encode(input, text_value(arguments, "delimiter")?, context)
            }
            Kind::A1z26Decode => {
                a1z26::a1z26_decode(input, text_value(arguments, "delimiter")?, context)
            }
            Kind::Affine(direction) => {
                let a = integer_value(arguments, "a")?;
                let b = integer_value(arguments, "b")?;
                match direction {
                    affine::Direction::Encode => affine::affine_encode(input, a, b, context),
                    affine::Direction::Decode => affine::affine_decode(input, a, b, context),
                }
            }
            Kind::Atbash => affine::atbash(input, context),
            Kind::CaesarBox => {
                transpose::caesar_box(input, integer_value(arguments, "box_height")?, context)
            }
            Kind::CetaceanEncode => whimsy::cetacean_encode(input, context),
            Kind::CetaceanDecode => whimsy::cetacean_decode(input, context),
            Kind::Leet => whimsy::leet(input, text_value(arguments, "direction")?, context),
            Kind::Nato => whimsy::to_nato(input, context),
            Kind::RailFenceEncode => transpose::rail_fence_encode(
                input,
                integer_value(arguments, "key")?,
                integer_value(arguments, "offset")?,
                context,
            ),
            Kind::RailFenceDecode => transpose::rail_fence_decode(
                input,
                integer_value(arguments, "key")?,
                integer_value(arguments, "offset")?,
                context,
            ),
            Kind::Rot8000 => rot::rot8000(input, context),
            Kind::Vigenere(shift) => {
                affine::vigenere(input, text_value(arguments, "key")?, *shift, context)
            }
        }
    }
}

impl Operation for ClassicalCipher {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = take_text(input)?;
        Ok(text(self.apply(&input, arguments, context)?))
    }
}

/// ROT47 over printable ASCII, which the reference runs on bytes.
pub struct Rot47 {
    spec: OperationSpec,
}

impl Rot47 {
    /// Creates the ROT47 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Bytes,
                UniformSpec {
                    id: "cipher.rot47@1",
                    display_name: "ROT47",
                    category: "Ciphers",
                    description: "Rotates the 94 printable ASCII characters.",
                    cyberchef_alias: "ROT47",
                    arguments: vec![integer_argument("amount", "Rotation amount.", 47)],
                },
            ),
        }
    }
}

impl Default for Rot47 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Rot47 {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let amount = integer_value(arguments, "amount")?;
        let input = take_bytes(input)?;
        Ok(bytes(rot::rot47(&input, amount, context)?))
    }
}
