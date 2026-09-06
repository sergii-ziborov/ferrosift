//! Pattern validation and evaluation through the real CLI process.

mod support;

#[test]
fn pattern_validate_accepts_a_supported_source() {
    let directory = support::TempDir::new("pattern-validate");
    let pattern = directory.write("header.hexpat", "be u16 magic @ 0x00;");
    let output = support::run(
        &[
            "pattern",
            "validate",
            "--pattern",
            support::path_text(&pattern),
        ],
        b"",
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout(&output).trim(), "valid");
}

#[test]
fn pattern_validate_rejects_syntax_errors_without_reading_input() {
    let directory = support::TempDir::new("pattern-invalid");
    let pattern = directory.write("broken.hexpat", "struct Broken {");
    let output = support::run(
        &[
            "pattern",
            "validate",
            "--pattern",
            support::path_text(&pattern),
        ],
        b"",
    );

    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("pattern.parse.unexpected_token"),
        "{}",
        support::stderr(&output)
    );
}

#[test]
fn pattern_run_returns_field_coordinates_as_json() {
    let directory = support::TempDir::new("pattern-run");
    let pattern = directory.write("header.hexpat", "be u16 magic @ 0x00;");
    let input = directory.write("payload.bin", [0x43, 0x41]);
    let output = support::run(
        &[
            "pattern",
            "run",
            "--pattern",
            support::path_text(&pattern),
            "--input",
            support::path_text(&input),
        ],
        b"",
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("pattern json");
    assert_eq!(report["schema"], "ferrosift.pattern.v1");
    assert_eq!(report["node_count"], 1);
    assert_eq!(report["nodes"][0]["name"], "magic");
    assert_eq!(report["nodes"][0]["offset"], 0);
    assert_eq!(report["nodes"][0]["size"], 2);
    assert_eq!(report["nodes"][0]["value"]["kind"], "unsigned");
    assert_eq!(report["nodes"][0]["value"]["value"], "17217");
}
