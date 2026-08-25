use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const SHORT_IPV4: &str = "network.strip.ipv4.too_short";
const SHORT_IPV4_IHL: &str = "network.strip.ipv4.shorter_than_ihl";
const SHORT_TCP: &str = "network.strip.tcp.too_short";
const SHORT_TCP_OFFSET: &str = "network.strip.tcp.shorter_than_offset";
const SHORT_UDP: &str = "network.strip.udp.too_short";

/// The shortest legal IPv4 and TCP headers, in bytes.
const MIN_HEADER: usize = 20;
/// UDP's header is a fixed four 16-bit fields.
const UDP_HEADER: usize = 8;

/// Drops an IPv4 header, returning the payload.
///
/// The header length is the low nibble of the first byte, counted in 32-bit
/// words. A packet whose declared header runs past its own end is refused
/// rather than truncated: the length field and the data disagree, and guessing
/// which to believe would invent a packet neither describes.
pub(super) fn strip_ipv4(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.len() < MIN_HEADER {
        return Err(failed(SHORT_IPV4));
    }
    let header = usize::from(input[0] & 0x0f) * 4;
    if input.len() < header {
        return Err(failed(SHORT_IPV4_IHL));
    }
    context.ensure_active()?;
    Ok(input[header..].to_vec())
}

/// Drops a TCP header, returning the payload.
///
/// The data offset is the high nibble of byte 12, counted in 32-bit words —
/// the same shape as IPv4's IHL but in a different place and a different half
/// of the byte.
pub(super) fn strip_tcp(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.len() < MIN_HEADER {
        return Err(failed(SHORT_TCP));
    }
    let header = usize::from(input[12] >> 4) * 4;
    if input.len() < header {
        return Err(failed(SHORT_TCP_OFFSET));
    }
    context.ensure_active()?;
    Ok(input[header..].to_vec())
}

/// Drops the fixed eight-byte UDP header, returning the payload.
pub(super) fn strip_udp(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    context.ensure_active()?;
    if input.len() < UDP_HEADER {
        return Err(failed(SHORT_UDP));
    }
    context.ensure_active()?;
    Ok(input[UDP_HEADER..].to_vec())
}
