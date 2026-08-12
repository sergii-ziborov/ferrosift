use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec;

fn text_out(value: alloc::string::String) -> Value {
    Value::Text(TextValue {
        text: value,
        encoding: TextEncoding::Utf8,
    })
}

fn require_text(input: Value) -> Result<alloc::string::String, OperationError> {
    match input {
        Value::Text(value) => Ok(value.text),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Defangs IPv4 / IPv6 addresses in text.
pub struct DefangIpAddresses {
    spec: OperationSpec,
}

impl DefangIpAddresses {
    /// Creates the IP defang operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "defang.ip@1",
                display_name: "Defang IP Addresses",
                category: "Defang",
                description: "Neutralises IPv4/IPv6 addresses by escaping dots and colons.",
                cyberchef_alias: Some("Defang IP Addresses"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
            }),
        }
    }
}

impl Default for DefangIpAddresses {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DefangIpAddresses {
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
        Ok(text_out(codec::defang_ip(&require_text(input)?, context)?))
    }
}

/// Defangs URLs and domains.
pub struct DefangUrl {
    spec: OperationSpec,
}

impl DefangUrl {
    /// Creates the URL defang operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "defang.url@1",
                display_name: "Defang URL",
                category: "Defang",
                description: "Neutralises URLs by escaping dots, http, and ://.",
                cyberchef_alias: Some("Defang URL"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    boolean_argument("escape_dots", "Replace '.' with '[.]'.", true),
                    boolean_argument("escape_http", "Replace 'http' with 'hxxp'.", true),
                    boolean_argument("escape_slashes", "Replace '://' with '[://]'.", true),
                    text_argument(
                        "process",
                        "Valid domains and full URLs, Only full URLs, or Everything.",
                        "Valid domains and full URLs",
                    ),
                ],
                inverse: Some("defang.fang_url@1"),
            }),
        }
    }
}

impl Default for DefangUrl {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for DefangUrl {
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
        Ok(text_out(codec::defang_url(
            &require_text(input)?,
            boolean_value(arguments, "escape_dots")?,
            boolean_value(arguments, "escape_http")?,
            boolean_value(arguments, "escape_slashes")?,
            text_value(arguments, "process")?,
            context,
        )?))
    }
}

/// Restores defanged URLs.
pub struct FangUrl {
    spec: OperationSpec,
}

impl FangUrl {
    /// Creates the fang URL operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "defang.fang_url@1",
                display_name: "Fang URL",
                category: "Defang",
                description: "Restores defanged URLs for analysis.",
                cyberchef_alias: Some("Fang URL"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    boolean_argument("restore_dots", "Restore '[.]' to '.'.", true),
                    boolean_argument("restore_hxxp", "Restore 'hxxp' to 'http'.", true),
                    boolean_argument("restore_slashes", "Restore '[://]' to '://'.", true),
                ],
                inverse: Some("defang.url@1"),
            }),
        }
    }
}

impl Default for FangUrl {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for FangUrl {
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
        Ok(text_out(codec::fang_url(
            &require_text(input)?,
            boolean_value(arguments, "restore_dots")?,
            boolean_value(arguments, "restore_hxxp")?,
            boolean_value(arguments, "restore_slashes")?,
            context,
        )?))
    }
}
