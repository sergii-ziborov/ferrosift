use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{map_argument, map_value, toggle_string_default, toggle_string_parts};
use crate::key::convert_to_byte_array;
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{bytes, take_bytes};

use super::codec::{self, Operator};

const INVALID_KEY: &str = "logic.bitwise.invalid_key";

/// A byte-wise logic or arithmetic operation against a repeating key.
///
/// The five reference operations differ only in the per-byte function and
/// whether they take a key at all, so they share one implementation rather
/// than five copies that could drift apart.
pub struct Bitwise {
    spec: OperationSpec,
    operator: Operator,
    keyed: bool,
}

impl Bitwise {
    /// Bitwise AND against the key.
    #[must_use]
    pub fn and() -> Self {
        Self::keyed(
            Operator::And,
            "logic.and@1",
            "AND",
            "AND",
            "Bitwise AND of every input byte with the repeating key.",
        )
    }

    /// Bitwise OR against the key.
    #[must_use]
    pub fn or() -> Self {
        Self::keyed(
            Operator::Or,
            "logic.or@1",
            "OR",
            "OR",
            "Bitwise OR of every input byte with the repeating key.",
        )
    }

    /// Byte-wise addition modulo 256.
    #[must_use]
    pub fn add() -> Self {
        Self::keyed(
            Operator::Add,
            "logic.add@1",
            "ADD",
            "ADD",
            "Adds the repeating key to every input byte, modulo 256.",
        )
    }

    /// Byte-wise subtraction modulo 256.
    #[must_use]
    pub fn sub() -> Self {
        Self::keyed(
            Operator::Sub,
            "logic.sub@1",
            "SUB",
            "SUB",
            "Subtracts the repeating key from every input byte, modulo 256.",
        )
    }

    /// Bitwise complement, which takes no key.
    #[must_use]
    pub fn not() -> Self {
        Self {
            spec: specification("logic.not@1", "NOT", "NOT", "Inverts every bit.", vec![]),
            operator: Operator::Not,
            keyed: false,
        }
    }

    fn keyed(
        operator: Operator,
        id: &'static str,
        display_name: &'static str,
        alias: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            spec: specification(
                id,
                display_name,
                alias,
                description,
                vec![map_argument(
                    "key",
                    "Repeating key as a toggle string.",
                    toggle_string_default("Hex", ""),
                )],
            ),
            operator,
            keyed: true,
        }
    }

    fn key(&self, arguments: &Arguments) -> Result<Vec<u8>, OperationError> {
        if !self.keyed {
            return Ok(Vec::new());
        }
        let (option, string) = toggle_string_parts(map_value(arguments, "key")?)?;
        convert_to_byte_array(string, option, INVALID_KEY)
    }
}

fn specification(
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

impl Operation for Bitwise {
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
        let key = self.key(arguments)?;
        let input = take_bytes(input)?;
        Ok(bytes(codec::bit_op(&input, &key, self.operator, context)?))
    }
}
