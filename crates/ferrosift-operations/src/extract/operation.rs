//! Helpers shared by every extractor.

use alloc::string::String;

use ferrosift_core::OperationError;
use ferrosift_model::{Arguments, TextEncoding, TextValue, Value};

use crate::args::boolean_value;

use super::common::PresentFlags;

pub(super) fn text_out(value: String) -> Value {
    Value::Text(TextValue {
        text: value,
        encoding: TextEncoding::Utf8,
    })
}

pub(super) fn require_text(input: Value) -> Result<String, OperationError> {
    match input {
        Value::Text(value) => Ok(value.text),
        _ => Err(OperationError::InvalidArguments),
    }
}

/// Reads the three presentation flags every extractor shares.
pub(super) fn present_flags(arguments: &Arguments) -> Result<PresentFlags, OperationError> {
    let mut bits = 0_u8;
    if boolean_value(arguments, "display_total")? {
        bits |= PresentFlags::DISPLAY_TOTAL;
    }
    if boolean_value(arguments, "sort")? {
        bits |= PresentFlags::SORT;
    }
    if boolean_value(arguments, "unique")? {
        bits |= PresentFlags::UNIQUE;
    }
    Ok(PresentFlags::from_bits(bits))
}

/// Declares the shared `display_total` / `sort` / `unique` arguments.
macro_rules! extract_flags {
    () => {
        alloc::vec![
            crate::args::boolean_argument(
                "display_total",
                "Prefix the result with a total count.",
                false,
            ),
            crate::args::boolean_argument("sort", "Sort matches.", false),
            crate::args::boolean_argument("unique", "Deduplicate matches.", false),
        ]
    };
}

pub(super) use extract_flags;
