use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::spec::{SpecDefinition, build};

use super::codec;

/// Which protocol header to drop.
#[derive(Clone, Copy)]
enum Layer {
    Ipv4,
    Tcp,
    Udp,
}

/// Drops a protocol header, leaving the payload.
pub struct StripHeader {
    spec: OperationSpec,
    layer: Layer,
}

impl StripHeader {
    /// Strips an IPv4 header.
    #[must_use]
    pub fn ipv4() -> Self {
        Self {
            spec: header_spec(
                "network.strip.ipv4@1",
                "Strip IPv4 header",
                "Drops the IPv4 header, leaving the payload.",
            ),
            layer: Layer::Ipv4,
        }
    }

    /// Strips a TCP header.
    #[must_use]
    pub fn tcp() -> Self {
        Self {
            spec: header_spec(
                "network.strip.tcp@1",
                "Strip TCP header",
                "Drops the TCP header, leaving the payload.",
            ),
            layer: Layer::Tcp,
        }
    }

    /// Strips a UDP header.
    #[must_use]
    pub fn udp() -> Self {
        Self {
            spec: header_spec(
                "network.strip.udp@1",
                "Strip UDP header",
                "Drops the eight-byte UDP header, leaving the payload.",
            ),
            layer: Layer::Udp,
        }
    }
}

fn header_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
) -> OperationSpec {
    build(SpecDefinition {
        id,
        display_name,
        category: "Networking",
        description,
        cyberchef_alias: Some(display_name),
        input: ValueConstraint::Exact(ValueKind::Bytes),
        output: ValueConstraint::Exact(ValueKind::Bytes),
        arguments: vec![],
        inverse: None,
        classifications: None,
    })
}

impl Operation for StripHeader {
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
        let output = match self.layer {
            Layer::Ipv4 => codec::strip_ipv4(&input, context)?,
            Layer::Tcp => codec::strip_tcp(&input, context)?,
            Layer::Udp => codec::strip_udp(&input, context)?,
        };
        Ok(Value::Bytes(output))
    }
}
