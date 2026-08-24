use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, UniformSpec, build, build_uniform};
use crate::value::{bytes, take_bytes, take_text, text};

use super::codec::{self, Direction};

fn byte_spec(
    id: &'static str,
    display_name: &'static str,
    cyberchef_alias: &'static str,
    description: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Bytes,
        UniformSpec {
            id,
            display_name,
            category: "Logic",
            description,
            cyberchef_alias,
            arguments,
        },
    )
}

/// Shifts every byte left or right by a fixed amount.
pub struct BitShift {
    spec: OperationSpec,
    left: bool,
}

impl BitShift {
    /// Shifts left, discarding the bits that fall off the top.
    #[must_use]
    pub fn left() -> Self {
        Self {
            spec: byte_spec(
                "logic.shift.left@1",
                "Bit shift left",
                "Bit shift left",
                "Shifts every byte left, discarding the overflow.",
                vec![integer_argument("amount", "Bits to shift by.", 1)],
            ),
            left: true,
        }
    }

    /// Shifts right, either logically or keeping the sign bit.
    #[must_use]
    pub fn right() -> Self {
        Self {
            spec: byte_spec(
                "logic.shift.right@1",
                "Bit shift right",
                "Bit shift right",
                "Shifts every byte right, logically or arithmetically.",
                vec![
                    integer_argument("amount", "Bits to shift by.", 1),
                    text_argument(
                        "shift_type",
                        "Logical shift or Arithmetic shift.",
                        "Logical shift",
                    ),
                ],
            ),
            left: false,
        }
    }
}

impl Operation for BitShift {
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
        let arithmetic = if self.left {
            false
        } else {
            text_value(arguments, "shift_type")? == "Arithmetic shift"
        };
        let input = take_bytes(input)?;
        let output = if self.left {
            codec::shift_left(&input, amount, context)?
        } else {
            codec::shift_right(&input, amount, arithmetic, context)?
        };
        Ok(bytes(output))
    }
}

/// Rotates bits within each byte, or across the whole buffer.
pub struct Rotate {
    spec: OperationSpec,
    direction: Direction,
}

impl Rotate {
    /// Rotates left.
    #[must_use]
    pub fn left() -> Self {
        Self::new(
            Direction::Left,
            "logic.rotate.left@1",
            "Rotate left",
            "Rotate left",
        )
    }

    /// Rotates right.
    #[must_use]
    pub fn right() -> Self {
        Self::new(
            Direction::Right,
            "logic.rotate.right@1",
            "Rotate right",
            "Rotate right",
        )
    }

    fn new(
        direction: Direction,
        id: &'static str,
        display_name: &'static str,
        cyberchef_alias: &'static str,
    ) -> Self {
        Self {
            spec: byte_spec(
                id,
                display_name,
                cyberchef_alias,
                "Rotates bits within each byte, or across the buffer when carrying.",
                vec![
                    integer_argument("amount", "Bits to rotate by.", 1),
                    boolean_argument("carry_through", "Carry bits across byte edges.", false),
                ],
            ),
            direction,
        }
    }
}

impl Operation for Rotate {
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
        let carry = boolean_value(arguments, "carry_through")?;
        let input = take_bytes(input)?;
        let output = if carry {
            codec::rotate_carry(&input, self.direction, amount, context)?
        } else {
            codec::rotate(&input, self.direction, amount, context)?
        };
        Ok(bytes(output))
    }
}

/// The ROR13 rolling hash, rendered as an upper-case `0x` word.
pub struct Ror13 {
    spec: OperationSpec,
}

impl Ror13 {
    /// Creates the ROR13 operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "logic.ror13@1",
                display_name: "ROR13",
                category: "Logic",
                description: "Computes the ROR13 rolling hash used to resolve API names.",
                cyberchef_alias: Some("ROR13"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Ror13 {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ror13 {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = take_bytes(input)?;
        Ok(text(codec::ror13(&input, context)?))
    }
}

/// Reverses byte order within fixed-width words.
pub struct SwapEndianness {
    spec: OperationSpec,
}

impl SwapEndianness {
    /// Creates the swap-endianness operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_uniform(
                ValueKind::Text,
                UniformSpec {
                    id: "data.swap_endianness@1",
                    display_name: "Swap endianness",
                    category: "Data",
                    description: "Reverses the byte order within each fixed-width word.",
                    cyberchef_alias: "Swap endianness",
                    arguments: vec![
                        text_argument("data_format", "Hex or Raw.", "Hex"),
                        integer_argument("word_length", "Word length in bytes.", 4),
                        boolean_argument("pad_incomplete_words", "Zero-pad a short word.", true),
                    ],
                },
            ),
        }
    }
}

impl Default for SwapEndianness {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for SwapEndianness {
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
        let format = text_value(arguments, "data_format")?;
        let word_length = integer_value(arguments, "word_length")?;
        let pad = boolean_value(arguments, "pad_incomplete_words")?;
        let input = take_text(input)?;
        Ok(text(codec::swap_endianness(
            &input,
            format,
            word_length,
            pad,
            context,
        )?))
    }
}
