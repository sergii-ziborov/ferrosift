//! CyberChef-aligned extract patterns via `regex-automata`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use regex_automata::{meta::Regex, util::syntax};

use super::common::{
    PresentFlags, SortKey, UniqueKey, ensure_output, finalize, finalize_with, format_results,
    format_results_labeled,
};
use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

const INVALID: &str = "extract.invalid_pattern";
const INVALID_HASH_LEN: &str = "extract.hash.invalid_length";

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
    // CyberChef's DOMAIN_REGEX caps each label at 63 characters with a
    // `(?=[a-z0-9-]{1,63}\.)` lookahead. `regex-automata` has no lookaround,
    // so the label body is kept verbatim and the length cap is dropped: the
    // result is identical for every label of 1..=63 characters (all real
    // domains), and the >63-character edge is a documented micro-divergence.
    let pattern = if dmarc {
        r"(?i)\b((xn--)?[a-z0-9_]+(-[a-z0-9_]+)*\.)+[a-z]{2,63}\b"
    } else {
        r"(?i)\b((xn--)?[a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,63}\b"
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

pub(super) fn extract_mac(
    input: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    // CyberChef: /[A-F\d]{2}(?:[:-][A-F\d]{2}){5}/ig
    const PATTERN: &str = r"(?i)[A-F0-9]{2}(?:[:-][A-F0-9]{2}){5}";
    context.ensure_active()?;
    let mut results = collect_matches(input, PATTERN)?;
    results = finalize_with(
        results,
        present.sort(),
        present.unique(),
        SortKey::Hexadecimal,
        UniqueKey::Exact,
        context,
    )?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}

pub(super) fn extract_file_paths(
    input: &str,
    windows: bool,
    unix: bool,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    // Port of CyberChef ExtractFilePaths patterns (case-insensitive).
    const WIN: &str = r"(?i)[A-Z]:\\(?:[A-Z0-9][A-Z0-9\- '_\(\)~]{0,61}\\?)*[A-Z0-9][A-Z0-9\- '_\(\)~]{0,61}(?:\.[A-Z0-9]{1,6})?";
    const UNIX: &str = r"(?i)(?:/[A-Z0-9.][A-Z0-9\-.]{0,61})+";
    context.ensure_active()?;
    let pattern = match (windows, unix) {
        (true, true) => {
            let mut combined = String::from("(?:");
            combined.push_str(WIN);
            combined.push('|');
            combined.push_str(UNIX);
            combined.push(')');
            combined
        }
        (true, false) => String::from(WIN),
        (false, true) => String::from(UNIX),
        (false, false) => {
            ensure_output("", context)?;
            return Ok(String::new());
        }
    };
    let mut results = collect_matches(input, &pattern)?;
    results = finalize(results, present.sort(), present.unique(), false, context)?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}

pub(super) fn extract_hashes(
    input: &str,
    hash_length: i128,
    all_hashes: bool,
    display_total: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if all_hashes {
        return extract_hash_lengths(
            input,
            &[
                4, 8, 16, 32, 64, 128, 160, 192, 224, 256, 320, 384, 512, 1024,
            ],
            display_total,
            context,
        );
    }
    if hash_length <= 0 || hash_length % 2 != 0 {
        return Err(failed(INVALID_HASH_LEN));
    }
    // Character length N => bit length (N/2)*8 = N*4.
    let bits =
        u32::try_from(hash_length.saturating_mul(4)).map_err(|_| failed(INVALID_HASH_LEN))?;
    extract_hash_lengths(input, &[bits], display_total, context)
}

fn extract_hash_lengths(
    input: &str,
    bit_lengths: &[u32],
    display_total: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let mut results = Vec::new();
    for &bits in bit_lengths {
        context.ensure_active()?;
        // Character length = (bits / 8) * 2.
        let chars = (bits / 8).saturating_mul(2);
        if chars == 0 {
            continue;
        }
        let mut pattern = String::new();
        // CyberChef: /(\b|^)[a-f0-9]{N}(\b|$)/g  (lowercase hex only)
        write!(&mut pattern, r"(\b|^)[a-f0-9]{{{chars}}}(\b|$)").map_err(|_| failed(INVALID))?;
        results.extend(collect_matches(input, &pattern)?);
    }
    let output = format_results_labeled(&results, display_total, "Total Results");
    ensure_output(&output, context)?;
    Ok(output)
}

fn extract_with(
    input: &str,
    pattern: &str,
    present: PresentFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let results = collect_matches(input, pattern)?;
    let results = finalize(results, present.sort(), present.unique(), false, context)?;
    let output = format_results(&results, present.display_total());
    ensure_output(&output, context)?;
    Ok(output)
}

fn collect_matches(input: &str, pattern: &str) -> Result<Vec<String>, OperationError> {
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
    Ok(results)
}
