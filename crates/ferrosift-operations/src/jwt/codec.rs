//! Decode a compact JWT into its JSON payload.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};
use ferrosift_model::StructuredValue;

use crate::base64;
use crate::failure::failed;

/// Decodes the JWT payload object. Header and signature are checked for shape
/// only; the signature is never verified.
pub(super) fn decode(
    token: &str,
    context: &mut OperationContext<'_>,
) -> Result<StructuredValue, OperationError> {
    context.ensure_active()?;
    let token = token.trim();
    let mut parts = token.split('.');
    let header = parts
        .next()
        .ok_or_else(|| failed("parsing.jwt.malformed"))?;
    let payload = parts
        .next()
        .ok_or_else(|| failed("parsing.jwt.malformed"))?;
    let signature = parts
        .next()
        .ok_or_else(|| failed("parsing.jwt.malformed"))?;
    if parts.next().is_some() || header.is_empty() || payload.is_empty() || signature.is_empty() {
        return Err(failed("parsing.jwt.malformed"));
    }

    // Header must be valid base64url JSON so a truncated token fails closed.
    let header_bytes = decode_segment(header, context)?;
    let _: serde_json::Value =
        serde_json::from_slice(&header_bytes).map_err(|_| failed("parsing.jwt.invalid_header"))?;

    let payload_bytes = decode_segment(payload, context)?;
    let value: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| failed("parsing.jwt.invalid_payload"))?;
    to_structured(value)
}

fn decode_segment(
    segment: &str,
    context: &mut OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    let padded = pad_base64url(segment);
    let mut standard = String::with_capacity(padded.len());
    for byte in padded.bytes() {
        match byte {
            b'-' => standard.push('+'),
            b'_' => standard.push('/'),
            other => standard.push(char::from(other)),
        }
    }
    base64::decode_standard(&standard, context).map_err(|_| failed("parsing.jwt.invalid_segment"))
}

fn pad_base64url(segment: &str) -> String {
    let remainder = segment.len() % 4;
    if remainder == 0 {
        return segment.to_owned();
    }
    let mut padded = String::with_capacity(segment.len() + (4 - remainder));
    padded.push_str(segment);
    for _ in 0..(4 - remainder) {
        padded.push('=');
    }
    padded
}

fn to_structured(value: serde_json::Value) -> Result<StructuredValue, OperationError> {
    match value {
        serde_json::Value::Null => Ok(StructuredValue::Null),
        serde_json::Value::Bool(flag) => Ok(StructuredValue::Boolean(flag)),
        serde_json::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(StructuredValue::Integer(i128::from(integer)))
            } else if let Some(integer) = number.as_u64() {
                Ok(StructuredValue::Integer(i128::from(integer)))
            } else if let Some(float) = number.as_f64() {
                // JWT claims are almost always integers; a non-integer float is
                // refused rather than silently truncated into i128.
                if float.fract() == 0.0 && float >= i128::MIN as f64 && float <= i128::MAX as f64 {
                    Ok(StructuredValue::Integer(float as i128))
                } else {
                    Err(failed("parsing.jwt.unsupported_number"))
                }
            } else {
                Err(failed("parsing.jwt.unsupported_number"))
            }
        }
        serde_json::Value::String(text) => Ok(StructuredValue::Text(text)),
        serde_json::Value::Array(values) => Ok(StructuredValue::List(
            values
                .into_iter()
                .map(to_structured)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        serde_json::Value::Object(map) => {
            let mut entries = Vec::with_capacity(map.len());
            for (key, nested) in map {
                entries.push((key, to_structured(nested)?));
            }
            Ok(StructuredValue::Object(entries))
        }
    }
}
