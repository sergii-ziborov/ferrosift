//! Container formats: bzip2 and gzip.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

// Both containers take a boolean argument; only bzip2 takes integers and only
// gzip takes text, so those two imports are gated and the shared pair is not.
use crate::args::{boolean_argument, boolean_value};
#[cfg(feature = "compression-bzip2")]
use crate::args::{integer_argument, integer_value};
#[cfg(feature = "compression-deflate")]
use crate::args::{text_argument, text_value};
#[cfg(feature = "compression-bzip2")]
use crate::spec::build_hosted;
use crate::spec::{SpecDefinition, build};

#[cfg(feature = "compression-bzip2")]
use super::bzip2;
#[cfg(feature = "compression-deflate")]
use super::codec;
/// Compresses data with Bzip2.
#[cfg(feature = "compression-bzip2")]
pub struct Bzip2Compress {
    spec: OperationSpec,
}

#[cfg(feature = "compression-bzip2")]
impl Bzip2Compress {
    /// Creates the Bzip2 compress operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_hosted(SpecDefinition {
                id: "compression.bzip2.compress@1",
                display_name: "Bzip2 Compress",
                category: "Compression",
                description: "Compresses data using the Bzip2 algorithm.",
                cyberchef_alias: Some("Bzip2 Compress"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    integer_argument(
                        "block_size",
                        "Block size in hundreds of kilobytes (1-9).",
                        9,
                    ),
                    integer_argument(
                        "work_factor",
                        "Accepted for CyberChef interchange; ignored by the portable encoder.",
                        30,
                    ),
                ],
                inverse: Some("compression.bzip2.decompress@1"),
                classifications: None,
            }),
        }
    }
}

#[cfg(feature = "compression-bzip2")]
impl Default for Bzip2Compress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "compression-bzip2")]
impl Operation for Bzip2Compress {
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
        Ok(Value::Bytes(bzip2::bzip2_compress(
            &input,
            integer_value(arguments, "block_size")?,
            integer_value(arguments, "work_factor")?,
            context,
        )?))
    }
}

/// Decompresses Bzip2 data.
#[cfg(feature = "compression-bzip2")]
pub struct Bzip2Decompress {
    spec: OperationSpec,
}

#[cfg(feature = "compression-bzip2")]
impl Bzip2Decompress {
    /// Creates the Bzip2 decompress operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_hosted(SpecDefinition {
                id: "compression.bzip2.decompress@1",
                display_name: "Bzip2 Decompress",
                category: "Compression",
                description: "Decompresses data using the Bzip2 algorithm.",
                cyberchef_alias: Some("Bzip2 Decompress"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![boolean_argument(
                    "low_memory",
                    "Accepted for CyberChef interchange; the portable decoder always uses the standard path.",
                    false,
                )],
                inverse: Some("compression.bzip2.compress@1"),
                classifications: None,
            }),
        }
    }
}

#[cfg(feature = "compression-bzip2")]
impl Default for Bzip2Decompress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "compression-bzip2")]
impl Operation for Bzip2Decompress {
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
        Ok(Value::Bytes(bzip2::bzip2_decompress(
            &input,
            boolean_value(arguments, "low_memory")?,
            context,
        )?))
    }
}

/// Decompresses gzip-wrapped deflate data.
#[cfg(feature = "compression-deflate")]
pub struct Gunzip {
    spec: OperationSpec,
}

#[cfg(feature = "compression-deflate")]
impl Gunzip {
    /// Creates the gunzip operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.gunzip@1",
                display_name: "Gunzip",
                category: "Compression",
                description: "Decompresses gzip-wrapped deflate data.",
                cyberchef_alias: Some("Gunzip"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![],
                inverse: Some("compression.gzip@1"),
                classifications: None,
            }),
        }
    }
}

#[cfg(feature = "compression-deflate")]
impl Default for Gunzip {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "compression-deflate")]
impl Operation for Gunzip {
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
        let input = crate::value::take_bytes(input)?;
        Ok(Value::Bytes(codec::gunzip(&input, context)?))
    }
}

/// Compresses data with gzip headers.
#[cfg(feature = "compression-deflate")]
pub struct Gzip {
    spec: OperationSpec,
}

#[cfg(feature = "compression-deflate")]
impl Gzip {
    /// Creates the gzip operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.gzip@1",
                display_name: "Gzip",
                category: "Compression",
                description: "Compresses data using deflate with gzip headers.",
                cyberchef_alias: Some("Gzip"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    text_argument(
                        "compression_type",
                        "Compression strategy token.",
                        "Dynamic Huffman Coding",
                    ),
                    text_argument("filename", "Optional original filename.", ""),
                    text_argument("comment", "Optional gzip comment.", ""),
                    boolean_argument(
                        "include_file_checksum",
                        "Include the optional gzip header CRC16.",
                        false,
                    ),
                ],
                inverse: Some("compression.gunzip@1"),
                classifications: None,
            }),
        }
    }
}

#[cfg(feature = "compression-deflate")]
impl Default for Gzip {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "compression-deflate")]
impl Operation for Gzip {
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
        Ok(Value::Bytes(codec::gzip(
            &input,
            text_value(arguments, "compression_type")?,
            text_value(arguments, "filename")?,
            text_value(arguments, "comment")?,
            boolean_value(arguments, "include_file_checksum")?,
            context,
        )?))
    }
}
