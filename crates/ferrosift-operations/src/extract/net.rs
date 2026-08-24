//! Network indicator extractors: IP addresses, URLs, and domains.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{boolean_argument, boolean_value};
use crate::spec::{SpecDefinition, build};

use super::operation::{extract_flags, present_flags, require_text, text_out};
use super::{ip, regexes};

/// Extracts IPv4 / IPv6 addresses.
pub struct ExtractIpAddresses {
    spec: OperationSpec,
}

impl ExtractIpAddresses {
    /// Creates the IP extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.ip@1",
                display_name: "Extract IP addresses",
                category: "Extractors",
                description: "Extracts IPv4 and IPv6 addresses from text.",
                cyberchef_alias: Some("Extract IP addresses"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    boolean_argument("ipv4", "Include IPv4 addresses.", true),
                    boolean_argument("ipv6", "Include IPv6 addresses.", false),
                    boolean_argument(
                        "remove_local_ipv4_addresses",
                        "Drop private / loopback IPv4 addresses.",
                        false,
                    ),
                    boolean_argument(
                        "display_total",
                        "Prefix the result with a total count.",
                        false,
                    ),
                    boolean_argument("sort", "Sort matches numerically for IPv4.", false),
                    boolean_argument("unique", "Deduplicate matches.", false),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractIpAddresses {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractIpAddresses {
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
        let mut ip_bits = 0_u8;
        if boolean_value(arguments, "ipv4")? {
            ip_bits |= ip::IpFlags::IPV4;
        }
        if boolean_value(arguments, "ipv6")? {
            ip_bits |= ip::IpFlags::IPV6;
        }
        if boolean_value(arguments, "remove_local_ipv4_addresses")? {
            ip_bits |= ip::IpFlags::REMOVE_LOCAL;
        }
        Ok(text_out(ip::extract(
            &input,
            ip::IpFlags::from_bits(ip_bits),
            present_flags(arguments)?,
            context,
        )?))
    }
}

/// Extracts full URLs that include a protocol.
pub struct ExtractUrls {
    spec: OperationSpec,
}

impl ExtractUrls {
    /// Creates the URL extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.url@1",
                display_name: "Extract URLs",
                category: "Extractors",
                description: "Extracts URLs with an explicit protocol.",
                cyberchef_alias: Some("Extract URLs"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: extract_flags!(),
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractUrls {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractUrls {
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
        Ok(text_out(regexes::extract_urls(
            &input,
            present_flags(arguments)?,
            context,
        )?))
    }
}

/// Extracts fully qualified domain names.
pub struct ExtractDomains {
    spec: OperationSpec,
}

impl ExtractDomains {
    /// Creates the domain extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.domain@1",
                display_name: "Extract domains",
                category: "Extractors",
                description: "Extracts fully qualified domain names.",
                cyberchef_alias: Some("Extract domains"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: {
                    let mut args = extract_flags!();
                    args.push(boolean_argument(
                        "underscore_dmarc_dkim",
                        "Allow underscores used by DMARC/DKIM labels.",
                        false,
                    ));
                    args
                },
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractDomains {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractDomains {
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
        Ok(text_out(regexes::extract_domains(
            &input,
            present_flags(arguments)?,
            boolean_value(arguments, "underscore_dmarc_dkim")?,
            context,
        )?))
    }
}
