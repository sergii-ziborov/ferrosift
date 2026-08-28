use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, CompatibilityProfile, OperationSpec, Value, ValueConstraint, ValueKind,
};

use crate::args::{integer_argument, integer_value};
use crate::spec::{SpecDefinition, build_since};

use super::codec;

/// Decompresses an XPRESS plain-LZ77 stream.
pub struct XpressDecompress {
    spec: OperationSpec,
}

impl XpressDecompress {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_since(
                CompatibilityProfile::CyberChefV11_4,
                SpecDefinition {
                    id: "compression.xpress.decompress@1",
                    display_name: "XPRESS Decompress",
                    category: "Compression",
                    description: "Decompresses data using the XPRESS plain LZ77 algorithm \
                                  (MS-XCA section 2.1).",
                    cyberchef_alias: Some("XPRESS Decompress"),
                    input: ValueConstraint::Exact(ValueKind::Bytes),
                    output: ValueConstraint::Exact(ValueKind::Bytes),
                    arguments: Vec::new(),
                    inverse: None,
                    classifications: None,
                },
            ),
        }
    }
}

impl Default for XpressDecompress {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for XpressDecompress {
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
        Ok(Value::Bytes(codec::decompress(&input, context)?))
    }
}

/// Decompresses an XPRESS LZ77+Huffman stream into a known size.
pub struct XpressHuffmanDecompress {
    spec: OperationSpec,
}

impl XpressHuffmanDecompress {
    /// Creates the operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_since(
                CompatibilityProfile::CyberChefV11_4,
                SpecDefinition {
                    id: "compression.xpress.huffman.decompress@1",
                    display_name: "XPRESS LZ77+Huffman Decompress",
                    category: "Compression",
                    description: "Decompresses data using the XPRESS LZ77+Huffman algorithm \
                                  (MS-XCA section 2.2).",
                    cyberchef_alias: Some("XPRESS LZ77+Huffman Decompress"),
                    input: ValueConstraint::Exact(ValueKind::Bytes),
                    output: ValueConstraint::Exact(ValueKind::Bytes),
                    // The stream does not say where it ends, so the size has
                    // to come from outside it — the WOF chunk table or the WIM
                    // header in the places this format is actually used.
                    arguments: vec![integer_argument(
                        "decompressed_size",
                        "Exact size of the decompressed data, which the stream does not carry.",
                        4096,
                    )],
                    inverse: None,
                    classifications: None,
                },
            ),
        }
    }
}

impl Default for XpressHuffmanDecompress {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for XpressHuffmanDecompress {
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
        let declared = integer_value(arguments, "decompressed_size")?;
        let input = crate::value::take_bytes(input)?;
        Ok(Value::Bytes(codec::decompress_huffman(
            &input, declared, context,
        )?))
    }
}
