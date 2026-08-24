//! Regression conformance for divergences the automatic corpus surfaced:
//! a previously-dead `Extract domains`, the hexdump final-length casing, and
//! the XOR Brute Force text sample formatting. All vectors are pinned against
//! `CyberChef` v11.3.0.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn arg_text(name: &str, value: &str) -> (String, ArgumentValue) {
    (name.into(), ArgumentValue::Text(value.into()))
}

fn arg_bool(name: &str, value: bool) -> (String, ArgumentValue) {
    (name.into(), ArgumentValue::Boolean(value))
}

#[test]
fn extract_domains_matches_reference_including_quirks() {
    // Standard mode extracts fully qualified names in match order, including
    // the CyberChef quirk that `cmd.exe` reads as a domain.
    let result = support::run(
        "extract.domain@1",
        Arguments::from([
            arg_bool("display_total", false),
            arg_bool("sort", false),
            arg_bool("unique", false),
            arg_bool("underscore_dmarc_dkim", false),
        ]),
        support::text("go to https://evil.example/x, mail a@corp.example.org, run cmd.exe"),
    );
    assert_eq!(
        support::output_text(result),
        "evil.example\ncorp.example.org\ncmd.exe"
    );
}

#[test]
fn extract_domains_dmarc_mode_allows_underscore_labels() {
    let result = support::run(
        "extract.domain@1",
        Arguments::from([
            arg_bool("display_total", false),
            arg_bool("sort", false),
            arg_bool("unique", false),
            arg_bool("underscore_dmarc_dkim", true),
        ]),
        support::text("_dmarc.example.com and plain.example.net"),
    );
    assert_eq!(
        support::output_text(result),
        "_dmarc.example.com\nplain.example.net"
    );
}

#[test]
fn extract_domains_honors_total_sort_and_unique() {
    let result = support::run(
        "extract.domain@1",
        Arguments::from([
            arg_bool("display_total", true),
            arg_bool("sort", true),
            arg_bool("unique", true),
            arg_bool("underscore_dmarc_dkim", false),
        ]),
        support::text("beta.example.org alpha.example.com beta.example.org"),
    );
    assert_eq!(
        support::output_text(result),
        "Total found: 2\n\nalpha.example.com\nbeta.example.org"
    );
}

#[test]
fn hexdump_final_length_line_stays_lowercase_in_upper_case_mode() {
    // CyberChef pushes the final-length line as raw lowercase hex after the
    // per-line upper-casing, so it must not be upper-cased.
    let result = support::run(
        "encoding.hexdump.encode@1",
        Arguments::from([
            ("width".into(), ArgumentValue::Integer(8)),
            arg_bool("upper_case_hex", true),
            arg_bool("include_final_length", true),
            arg_bool("unix_format", false),
        ]),
        Value::Bytes(vec![0xab, 0xcd, 0xef]),
    );
    // The hex field is padded to width*3 columns, the ASCII gutter keeps the
    // Latin-1 bytes, and the final-length trailer stays lowercase.
    let expected = format!(
        "00000000  {:<24} |\u{ab}\u{cd}\u{ef}|\n00000003",
        "AB CD EF"
    );
    assert_eq!(support::output_text(result), expected);
}

#[test]
fn xor_brute_force_text_escapes_control_bytes_like_the_reference() {
    // The text sample is UTF-8 decoded (Latin-1 fallback) and control bytes
    // 0x09..=0x10 are shifted into the U+E000 private-use area so a decoded
    // newline cannot corrupt the line-delimited report.
    let result = support::run(
        "logic.xor_brute@1",
        Arguments::from([
            ("key_length".into(), ArgumentValue::Integer(1)),
            ("sample_length".into(), ArgumentValue::Integer(4)),
            ("sample_offset".into(), ArgumentValue::Integer(0)),
            arg_text("scheme", "Standard"),
            arg_bool("null_preserving", false),
            arg_bool("print_key", true),
            arg_bool("output_as_hex", false),
            arg_text("crib", ""),
        ]),
        // 0x0b ^ 0x01 = 0x0a (newline): escaped, not emitted literally.
        Value::Bytes(vec![0x0b, 0x41, 0x42, 0x43]),
    );
    let text = support::output_text(result);
    let first = text.lines().next().expect("at least one key line");
    assert_eq!(first, "Key = 01: \u{e00a}@CB");
    assert!(
        !first.contains('\n'),
        "control bytes must not break the record into extra lines"
    );
}
