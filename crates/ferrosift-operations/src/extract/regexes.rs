//! CyberChef-aligned extract patterns via `regex-automata`.

use alloc::string::String;
use alloc::vec::Vec;

use regex_automata::{meta::Regex, util::syntax};

use super::common::{PresentFlags, ensure_output, finalize, format_results};
use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID: &str = "extract.invalid_pattern";

pub(super) fn extract_urls(
    input: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    const PATTERN: &str = r#"(?i)[A-Z]+://[-\w]+(?:\.\w[-\w]*)+(?::\d+)?(?:/[^.!,?"<>\[\]{}\s\x7F-\u{00FF}]*(?:[.!,?]+[^.!,?"<>\[\]{}\s\x7F-\u{00FF}]+)*)?"#;
    extract_with(input, PATTERN, present, context)
}

pub(super) fn extract_domains(
    input: &str,
    present: PresentFlags,
    dmarc: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let pattern = if dmarc {
        r"(?i)\b((?=[a-z0-9_-]{1,63}\.)(xn--)?[a-z0-9_]+(-[a-z0-9_]+)*\.)+[a-z]{2,63}\b"
    } else {
        r"(?i)\b((?=[a-z0-9-]{1,63}\.)(xn--)?[a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,63}\b"
    };
    extract_with(input, pattern, present, context)
}

pub(super) fn extract_emails(
    input: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    const PATTERN: &str = r"(?i)[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?";
    extract_with(input, PATTERN, present, context)
}

fn extract_with(
    input: &str,
    pattern: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let regex = Regex::builder()
        .syntax(syntax::Config::new().unicode(true).utf8(true))
        .build(pattern)
        .map_err(|_| failed(INVALID))?;
    let mut results = Vec::new();
    for found in regex.find_iter(input.as_bytes()) {
        if !input.is_char_boundary(found.start()) || !input.is_char_boundary(found.end()) {
            continue;
        }
        results.push(String::from(&input[found.start()..found.end()]));
    }
    let results = finalize(results, present.sort(), present.unique(), false, context)?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}
