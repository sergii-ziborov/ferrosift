//! Raw and zlib DEFLATE operations.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, build};

use super::codec;
/// Compresses data with raw deflate (no headers).
pub struct RawDeflate {
    spec: OperationSpec,
}

impl RawDeflate {
    /// Creates the raw deflate operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.raw.deflate@1",
                display_name: "Raw Deflate",
                category: "Compression",
                description: "Compresses data using deflate with no headers.",
                cyberchef_alias: Some("Raw Deflate"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![text_argument(
                    "compression_type",
                    "Compression strategy token.",
                    "Dynamic Huffman Coding",
                )],
                inverse: Some("compression.raw.inflate@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for RawDeflate {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RawDeflate {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::raw_deflate(
            &input,
            text_value(arguments, "compression_type")?,
            context,
        )?))
    }
}

/// Decompresses raw deflate data (no headers).
pub struct RawInflate {
    spec: OperationSpec,
}

impl RawInflate {
    /// Creates the raw inflate operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.raw.inflate@1",
                display_name: "Raw Inflate",
                category: "Compression",
                description: "Decompresses raw deflate data with no headers.",
                cyberchef_alias: Some("Raw Inflate"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    integer_argument("start_index", "Byte offset into the input.", 0),
                    integer_argument(
                        "initial_output_buffer_size",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        0,
                    ),
                    text_argument(
                        "buffer_expansion_type",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        "Adaptive",
                    ),
                    boolean_argument(
                        "resize_buffer_after_decompression",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        false,
                    ),
                    boolean_argument(
                        "verify_result",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        false,
                    ),
                ],
                inverse: Some("compression.raw.deflate@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for RawInflate {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RawInflate {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        let _ = (
            integer_value(arguments, "initial_output_buffer_size")?,
            text_value(arguments, "buffer_expansion_type")?,
            boolean_value(arguments, "resize_buffer_after_decompression")?,
            boolean_value(arguments, "verify_result")?,
        );
        Ok(Value::Bytes(codec::raw_inflate(
            &input,
            integer_value(arguments, "start_index")?,
            context,
        )?))
    }
}

/// Compresses data with zlib headers.
pub struct ZlibDeflate {
    spec: OperationSpec,
}

impl ZlibDeflate {
    /// Creates the zlib deflate operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.zlib.deflate@1",
                display_name: "Zlib Deflate",
                category: "Compression",
                description: "Compresses data using deflate with zlib headers.",
                cyberchef_alias: Some("Zlib Deflate"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![text_argument(
                    "compression_type",
                    "Compression strategy token.",
                    "Dynamic Huffman Coding",
                )],
                inverse: Some("compression.zlib.inflate@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ZlibDeflate {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ZlibDeflate {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        Ok(Value::Bytes(codec::zlib_deflate(
            &input,
            text_value(arguments, "compression_type")?,
            context,
        )?))
    }
}

/// Decompresses zlib-wrapped deflate data.
pub struct ZlibInflate {
    spec: OperationSpec,
}

impl ZlibInflate {
    /// Creates the zlib inflate operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "compression.zlib.inflate@1",
                display_name: "Zlib Inflate",
                category: "Compression",
                description: "Decompresses zlib-wrapped deflate data.",
                cyberchef_alias: Some("Zlib Inflate"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Bytes),
                arguments: vec![
                    integer_argument("start_index", "Byte offset into the input.", 0),
                    integer_argument(
                        "initial_output_buffer_size",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        0,
                    ),
                    text_argument(
                        "buffer_expansion_type",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        "Adaptive",
                    ),
                    boolean_argument(
                        "resize_buffer_after_decompression",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        false,
                    ),
                    boolean_argument(
                        "verify_result",
                        "Ignored portable placeholder matching the CyberChef argument slot.",
                        false,
                    ),
                ],
                inverse: Some("compression.zlib.deflate@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for ZlibInflate {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ZlibInflate {
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
        let Value::Bytes(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        // Extra CyberChef buffer knobs are accepted for interchange and ignored.
        let _ = (
            integer_value(arguments, "initial_output_buffer_size")?,
            text_value(arguments, "buffer_expansion_type")?,
            boolean_value(arguments, "resize_buffer_after_decompression")?,
            boolean_value(arguments, "verify_result")?,
        );
        Ok(Value::Bytes(codec::zlib_inflate(
            &input,
            integer_value(arguments, "start_index")?,
            context,
        )?))
    }
}
