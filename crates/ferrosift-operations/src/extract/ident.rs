//! Identity extractors: email and MAC addresses.

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

use super::operation::{extract_flags, present_flags, require_text, text_out};
use super::regexes;

/// Extracts email addresses.
pub struct ExtractEmailAddresses {
    spec: OperationSpec,
}

impl ExtractEmailAddresses {
    /// Creates the email extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.email@1",
                display_name: "Extract email addresses",
                category: "Extractors",
                description: "Extracts email addresses from text.",
                cyberchef_alias: Some("Extract email addresses"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: extract_flags!(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractEmailAddresses {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractEmailAddresses {
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
        let input = require_text(input)?;
        Ok(text_out(regexes::extract_emails(
            &input,
            present_flags(arguments)?,
            context,
        )?))
    }
}

/// Extracts Media Access Control addresses.
pub struct ExtractMacAddresses {
    spec: OperationSpec,
}

impl ExtractMacAddresses {
    /// Creates the MAC extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.mac@1",
                display_name: "Extract MAC addresses",
                category: "Extractors",
                description: "Extracts MAC addresses from text.",
                cyberchef_alias: Some("Extract MAC addresses"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: extract_flags!(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractMacAddresses {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractMacAddresses {
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
        let input = require_text(input)?;
        Ok(text_out(regexes::extract_mac(
            &input,
            present_flags(arguments)?,
            context,
        )?))
    }
}
