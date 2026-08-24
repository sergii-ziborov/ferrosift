//! Operation wrappers for the checksums.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{integer_argument, integer_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_bytes, take_text, text};

use super::codec::{self, Fletcher};

/// Builds a checksum specification, which always reports as text.
fn checksum_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    input: ValueKind,
    arguments: alloc::vec::Vec<ArgumentSpec>,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Checksums",
        description,
        cyberchef_alias: Some(display_name),
        input: ValueConstraint::Exact(input),
        output: ValueConstraint::Exact(ValueKind::Text),
        arguments,
        inverse: None,
        classifications: None,
    })
}

/// A byte-input checksum reported as a hex string.
pub struct Checksum {
    spec: OperationSpec,
    kind: Kind,
}

enum Kind {
    Adler32,
    Fletcher(Fletcher),
    TcpIp,
    Xor,
}

impl Checksum {
    /// Adler-32.
    #[must_use]
    pub fn adler32() -> Self {
        Self {
            spec: checksum_spec(
                "checksum.adler32@1",
                "Adler-32 Checksum",
                "Computes the Adler-32 checksum of the input.",
                ValueKind::Bytes,
                vec![],
            ),
            kind: Kind::Adler32,
        }
    }

    /// Fletcher-8.
    #[must_use]
    pub fn fletcher8() -> Self {
        Self::fletcher(
            Fletcher::Eight,
            "checksum.fletcher8@1",
            "Fletcher-8 Checksum",
        )
    }

    /// Fletcher-16.
    #[must_use]
    pub fn fletcher16() -> Self {
        Self::fletcher(
            Fletcher::Sixteen,
            "checksum.fletcher16@1",
            "Fletcher-16 Checksum",
        )
    }

    /// Fletcher-32.
    #[must_use]
    pub fn fletcher32() -> Self {
        Self::fletcher(
            Fletcher::ThirtyTwo,
            "checksum.fletcher32@1",
            "Fletcher-32 Checksum",
        )
    }

    /// Fletcher-64.
    #[must_use]
    pub fn fletcher64() -> Self {
        Self::fletcher(
            Fletcher::SixtyFour,
            "checksum.fletcher64@1",
            "Fletcher-64 Checksum",
        )
    }

    fn fletcher(width: Fletcher, id: &'static str, display_name: &'static str) -> Self {
        Self {
            spec: checksum_spec(
                id,
                display_name,
                "Computes the Fletcher checksum of the input.",
                ValueKind::Bytes,
                vec![],
            ),
            kind: Kind::Fletcher(width),
        }
    }

    /// TCP/IP header checksum.
    #[must_use]
    pub fn tcp_ip() -> Self {
        Self {
            spec: checksum_spec(
                "checksum.tcp_ip@1",
                "TCP/IP Checksum",
                "Computes the one's-complement TCP/IP header checksum.",
                ValueKind::Bytes,
                vec![],
            ),
            kind: Kind::TcpIp,
        }
    }

    /// Block-wise XOR checksum.
    #[must_use]
    pub fn xor() -> Self {
        Self {
            spec: checksum_spec(
                "checksum.xor@1",
                "XOR Checksum",
                "XORs the input together in fixed-size blocks.",
                ValueKind::Bytes,
                vec![integer_argument("blocksize", "Block size in bytes.", 4)],
            ),
            kind: Kind::Xor,
        }
    }
}

impl Operation for Checksum {
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
        let block_size = match self.kind {
            Kind::Xor => integer_value(arguments, "blocksize")?,
            _ => 0,
        };
        let input = take_bytes(input)?;
        let output = match self.kind {
            Kind::Adler32 => codec::adler32(&input, context)?,
            Kind::Fletcher(width) => codec::fletcher(&input, width, context)?,
            Kind::TcpIp => codec::tcp_ip(&input, context)?,
            Kind::Xor => codec::xor_checksum(&input, block_size, context)?,
        };
        Ok(text(output))
    }
}

/// The Luhn checksum, which reads text rather than bytes.
pub struct LuhnChecksum {
    spec: OperationSpec,
}

impl LuhnChecksum {
    /// Creates the Luhn checksum operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: checksum_spec(
                "checksum.luhn@1",
                "Luhn Checksum",
                "Computes the Luhn checksum, check digit, and validated string.",
                ValueKind::Text,
                vec![integer_argument("radix", "Radix, even and 2 to 36.", 10)],
            ),
        }
    }
}

impl Default for LuhnChecksum {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for LuhnChecksum {
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
        let radix = integer_value(arguments, "radix")?;
        let input = take_text(input)?;
        Ok(text(codec::luhn(&input, radix, context)?))
    }
}
