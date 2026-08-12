//! Conformance for extractors, defang/fang, and strings.

use ferrosift_model::{ArgumentValue, Arguments};

mod support;

const SAMPLE: &str = "Contact admin@example.com or visit https://evil.example/path?x=1 see 8.8.8.8 and 192.168.1.1 also domain.example.org";

#[test]
fn extracts_ips_urls_emails_and_domains() {
    assert_eq!(
        support::output_text(support::run(
            "extract.ip@1",
            Arguments::from([
                ("ipv4".into(), ArgumentValue::Boolean(true)),
                ("ipv6".into(), ArgumentValue::Boolean(false)),
                (
                    "remove_local_ipv4_addresses".into(),
                    ArgumentValue::Boolean(false),
                ),
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(SAMPLE),
        )),
        "8.8.8.8\n192.168.1.1"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.ip@1",
            Arguments::from([
                ("ipv4".into(), ArgumentValue::Boolean(true)),
                ("ipv6".into(), ArgumentValue::Boolean(false)),
                (
                    "remove_local_ipv4_addresses".into(),
                    ArgumentValue::Boolean(true),
                ),
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(true)),
            ]),
            support::text(SAMPLE),
        )),
        "8.8.8.8"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.url@1",
            Arguments::from([
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(SAMPLE),
        )),
        "https://evil.example/path?x=1"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.email@1",
            Arguments::from([
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(SAMPLE),
        )),
        "admin@example.com"
    );
}

#[test]
fn defang_and_fang_round_trip_urls_and_ips() {
    let defanged = support::output_text(support::run(
        "defang.url@1",
        Arguments::from([
            ("escape_dots".into(), ArgumentValue::Boolean(true)),
            ("escape_http".into(), ArgumentValue::Boolean(true)),
            ("escape_slashes".into(), ArgumentValue::Boolean(true)),
            (
                "process".into(),
                ArgumentValue::Text("Only full URLs".into()),
            ),
        ]),
        support::text("https://evil.example/path"),
    ));
    assert_eq!(defanged, "hxxps[://]evil[.]example/path");
    assert_eq!(
        support::output_text(support::run(
            "defang.fang_url@1",
            Arguments::from([
                ("restore_dots".into(), ArgumentValue::Boolean(true)),
                ("restore_hxxp".into(), ArgumentValue::Boolean(true)),
                ("restore_slashes".into(), ArgumentValue::Boolean(true)),
            ]),
            support::text(&defanged),
        )),
        "https://evil.example/path"
    );
    assert_eq!(
        support::output_text(support::run(
            "defang.ip@1",
            Arguments::new(),
            support::text("8.8.8.8 and 1.2.3.4"),
        )),
        "8[.]8[.]8[.]8 and 1[.]2[.]3[.]4"
    );
}

#[test]
fn strings_extracts_ascii_printable_runs() {
    assert_eq!(
        support::output_text(support::run(
            "extract.strings@1",
            Arguments::from([
                ("encoding".into(), ArgumentValue::Text("Single byte".into()),),
                ("minimum_length".into(), ArgumentValue::Integer(4)),
                (
                    "match".into(),
                    ArgumentValue::Text("All printable chars (A)".into()),
                ),
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text("\u{0}\u{0}Hello World\u{0}\u{0}test\u{0}AB"),
        )),
        "Hello World\ntest"
    );
}
