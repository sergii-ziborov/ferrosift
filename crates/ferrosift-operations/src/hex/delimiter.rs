use ferrosift_core::{OperationError, OperationFailureCode};

#[derive(Clone, Copy)]
pub(super) enum EncodeDelimiter {
    Suffix(&'static str),
    Prefix(&'static str),
    PrefixWithComma,
}

#[derive(Clone, Copy)]
pub(super) enum DecodeDelimiter {
    Auto,
    Compact,
    Whitespace,
    Separated(&'static str),
    Prefixed(&'static str),
    PrefixedWithComma,
}

pub(super) fn encode(value: &str) -> Result<EncodeDelimiter, OperationError> {
    match value {
        "Space" => Ok(EncodeDelimiter::Suffix(" ")),
        "Percent" => Ok(EncodeDelimiter::Prefix("%")),
        "Comma" => Ok(EncodeDelimiter::Suffix(",")),
        "Semi-colon" => Ok(EncodeDelimiter::Suffix(";")),
        "Colon" => Ok(EncodeDelimiter::Suffix(":")),
        "Line feed" => Ok(EncodeDelimiter::Suffix("\n")),
        "CRLF" => Ok(EncodeDelimiter::Suffix("\r\n")),
        "0x" => Ok(EncodeDelimiter::Prefix("0x")),
        "0x with comma" => Ok(EncodeDelimiter::PrefixWithComma),
        "\\x" => Ok(EncodeDelimiter::Prefix("\\x")),
        "None" => Ok(EncodeDelimiter::Suffix("")),
        _ => Err(failed("encoding.hex.invalid_delimiter")),
    }
}

pub(super) fn decode(value: &str) -> Result<DecodeDelimiter, OperationError> {
    match value {
        "Auto" => Ok(DecodeDelimiter::Auto),
        "None" => Ok(DecodeDelimiter::Compact),
        "Space" => Ok(DecodeDelimiter::Whitespace),
        "Percent" => Ok(DecodeDelimiter::Prefixed("%")),
        "Comma" => Ok(DecodeDelimiter::Separated(",")),
        "Semi-colon" => Ok(DecodeDelimiter::Separated(";")),
        "Colon" => Ok(DecodeDelimiter::Separated(":")),
        "Line feed" => Ok(DecodeDelimiter::Separated("\n")),
        "CRLF" => Ok(DecodeDelimiter::Separated("\r\n")),
        "0x" => Ok(DecodeDelimiter::Prefixed("0x")),
        "0x with comma" => Ok(DecodeDelimiter::PrefixedWithComma),
        "\\x" => Ok(DecodeDelimiter::Prefixed("\\x")),
        _ => Err(failed("encoding.hex.invalid_delimiter")),
    }
}

pub(super) fn failed(value: &'static str) -> OperationError {
    OperationError::Failed {
        code: OperationFailureCode::from_static(value),
    }
}
