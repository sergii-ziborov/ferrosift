use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

/// Percent-encodes bytes as URL-safe text.
pub struct UrlEncode {
    spec: OperationSpec,
}

impl UrlEncode {
    /// Creates the URL encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.url.encode@1",
                display_name: "URL Encode",
                category: "Encoding",
                description: "Percent-encodes bytes as URL text.",
                cyberchef_alias: Some("URL Encode"),
                input: ValueConstraint::Exact(ValueKind::Bytes),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![boolean_argument(
                    "encode_all_special_chars",
                    "Percent-encode every non-alphanumeric byte.",
                    false,
                )],
                inverse: Some("encoding.url.decode@1"),
            }),
        }
    }
}

impl Default for UrlEncode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UrlEncode {
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
        let output = codec::encode(
            &input,
            boolean_value(arguments, "encode_all_special_chars")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes percent-encoded URL text back into text.
pub struct UrlDecode {
    spec: OperationSpec,
}

impl UrlDecode {
    /// Creates the URL decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "encoding.url.decode@1",
                display_name: "URL Decode",
                category: "Encoding",
                description: "Decodes percent-encoded URL text.",
                cyberchef_alias: Some("URL Decode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![boolean_argument(
                    "treat_plus_as_space",
                    "Decode + characters as spaces.",
                    true,
                )],
                inverse: Some("encoding.url.encode@1"),
            }),
        }
    }
}

impl Default for UrlDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for UrlDecode {
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
        let Value::Text(input) = input else {
            return Err(OperationError::InvalidArguments);
        };
        let output = codec::decode(
            &input.text,
            boolean_value(arguments, "treat_plus_as_space")?,
            context,
        )?;
        Ok(Value::Text(TextValue {
            text: output,
            encoding: TextEncoding::Utf8,
        }))
    }
}
