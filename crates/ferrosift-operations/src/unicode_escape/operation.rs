use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{bytes as bytes_output, take_bytes, take_text, text as text_output};

use super::codec;

fn text_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Text,
        UniformSpec {
            id,
            display_name,
            category: "Encoding",
            description,
            cyberchef_alias: alias,
            arguments,
        },
    )
}

fn bytes_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Bytes,
        UniformSpec {
            id,
            display_name,
            category: "Encoding",
            description,
            cyberchef_alias: alias,
            arguments,
        },
    )
}

/// Escapes characters as Unicode escapes.
pub struct EscapeUnicodeCharacters {
    spec: OperationSpec,
}

impl EscapeUnicodeCharacters {
    /// Creates the escaping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "encoding.unicode.escape@1",
                "Escape Unicode Characters",
                "Replaces characters with Unicode escapes such as \\u0041.",
                "Escape Unicode Characters",
                vec![
                    text_argument("prefix", "Escape prefix: \\u, %u, or U+.", "\\u"),
                    boolean_argument(
                        "encode_all_chars",
                        "Escape printable ASCII as well as everything else.",
                        false,
                    ),
                    integer_argument("padding", "Minimum hex digits per escape.", 4),
                    boolean_argument("uppercase_hex", "Emit hex digits in upper case.", true),
                ],
            ),
        }
    }
}

impl Default for EscapeUnicodeCharacters {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for EscapeUnicodeCharacters {
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
        let prefix = codec::prefix(text_value(arguments, "prefix")?)?;
        let encode_all = boolean_value(arguments, "encode_all_chars")?;
        let upper = boolean_value(arguments, "uppercase_hex")?;
        // A negative padding pads nothing, exactly as `padStart` treats it.
        let padding = usize::try_from(integer_value(arguments, "padding")?).unwrap_or(0);
        let input = take_text(input)?;
        Ok(text_output(codec::escape(
            &input, prefix, encode_all, padding, upper,
        )))
    }
}

/// Replaces Unicode escapes with the characters they name.
pub struct UnescapeUnicodeCharacters {
    spec: OperationSpec,
}

impl UnescapeUnicodeCharacters {
    /// Creates the unescaping operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "encoding.unicode.unescape@1",
                "Unescape Unicode Characters",
                "Replaces Unicode escapes such as \\u0041 with their characters.",
                "Unescape Unicode Characters",
                vec![text_argument(
                    "prefix",
                    "Escape prefix: \\u, %u, or U+.",
                    "\\u",
                )],
            ),
        }
    }
}

impl Default for UnescapeUnicodeCharacters {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UnescapeUnicodeCharacters {
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
        let prefix = codec::prefix(text_value(arguments, "prefix")?)?;
        let input = take_text(input)?;
        Ok(text_output(codec::unescape(&input, prefix)))
    }
}

/// Encodes a `NetBIOS` name.
pub struct EncodeNetbiosName {
    spec: OperationSpec,
}

impl EncodeNetbiosName {
    /// Creates the `NetBIOS` encoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: bytes_spec(
                "encoding.netbios.encode@1",
                "Encode NetBIOS Name",
                "Encodes a NetBIOS name as nibble pairs offset from a base byte.",
                "Encode NetBIOS Name",
                vec![integer_argument("offset", "Base byte for each nibble.", 65)],
            ),
        }
    }
}

impl Default for EncodeNetbiosName {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for EncodeNetbiosName {
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
        let offset = offset_byte(arguments)?;
        let input = take_bytes(input)?;
        Ok(bytes_output(codec::netbios_encode(&input, offset)))
    }
}

/// Decodes a `NetBIOS` name.
pub struct DecodeNetbiosName {
    spec: OperationSpec,
}

impl DecodeNetbiosName {
    /// Creates the `NetBIOS` decoding operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: bytes_spec(
                "encoding.netbios.decode@1",
                "Decode NetBIOS Name",
                "Decodes a NetBIOS name from nibble pairs offset from a base byte.",
                "Decode NetBIOS Name",
                vec![integer_argument("offset", "Base byte for each nibble.", 65)],
            ),
        }
    }
}

impl Default for DecodeNetbiosName {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DecodeNetbiosName {
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
        let offset = offset_byte(arguments)?;
        let input = take_bytes(input)?;
        Ok(bytes_output(codec::netbios_decode(&input, offset)?))
    }
}

/// The nibble offset, which the reference adds to a byte and wraps.
fn offset_byte(arguments: &Arguments) -> Result<u8, OperationError> {
    let value = integer_value(arguments, "offset")?;
    u8::try_from(value.rem_euclid(256)).map_err(|_| OperationError::InvalidArguments)
}
