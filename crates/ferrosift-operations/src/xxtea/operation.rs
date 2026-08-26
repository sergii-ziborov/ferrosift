use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{map_argument, map_value, toggle_string_default, toggle_string_parts};
use crate::key::{XOR_INVALID_KEY, convert_to_byte_array};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Which direction of the cipher an instance runs.
#[derive(Clone, Copy)]
enum Direction {
    Encrypt,
    Decrypt,
}

/// XXTEA, the corrected Block TEA.
pub struct Xxtea {
    spec: OperationSpec,
    direction: Direction,
}

impl Xxtea {
    /// Creates the encrypting half.
    #[must_use]
    pub fn encrypt() -> Self {
        Self::new(Direction::Encrypt)
    }

    /// Creates the decrypting half.
    #[must_use]
    pub fn decrypt() -> Self {
        Self::new(Direction::Decrypt)
    }

    fn new(direction: Direction) -> Self {
        let (id, name, inverse, description) = match direction {
            Direction::Encrypt => (
                "crypto.xxtea.encrypt@1",
                "XXTEA Encrypt",
                "crypto.xxtea.decrypt@1",
                "Encrypts bytes with the XXTEA block cipher.",
            ),
            Direction::Decrypt => (
                "crypto.xxtea.decrypt@1",
                "XXTEA Decrypt",
                "crypto.xxtea.encrypt@1",
                "Decrypts bytes encrypted with the XXTEA block cipher.",
            ),
        };
        Self {
            spec: build(SpecDefinition {
                id,
                display_name: name,
                category: "Ciphers",
                description,
                cyberchef_alias: Some(name),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![map_argument(
                    "key",
                    "Key value and encoding (CyberChef toggleString).",
                    toggle_string_default("Hex", ""),
                )],
                inverse: Some(inverse),
                classifications: None,
            }),
            direction,
        }
    }
}

impl Operation for Xxtea {
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
        let input = crate::value::take_bytes(input)?;
        let (option, string) = toggle_string_parts(map_value(arguments, "key")?)?;
        let key = convert_to_byte_array(string, option, XOR_INVALID_KEY)?;
        let output = match self.direction {
            Direction::Encrypt => codec::encrypt(&input, &key, context)?,
            Direction::Decrypt => codec::decrypt(&input, &key, context)?,
        };
        Ok(Value::Bytes(output))
    }
}
