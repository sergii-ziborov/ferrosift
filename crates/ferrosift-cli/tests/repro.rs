//! Reproducible case export and replay through the real CLI.

use std::fs;

mod support;

#[test]
fn repro_export_then_check_passes_on_a_clean_replay() {
    let directory = support::TempDir::new("repro");
    let recipe = directory.write("recipe.json", r#"[{"op":"To Hex","args":["Space",0]}]"#);
    let input = directory.write("input.bin", b"Hi");
    let case_dir = directory.path("payload-case");

    let export = support::run(
        &[
            "repro",
            "export",
            "--format",
            "cyberchef-v11.3",
            "--input-kind",
            "bytes",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            support::path_text(&input),
            "--out-dir",
            support::path_text(&case_dir),
        ],
        b"",
    );
    assert!(export.status.success(), "{}", support::stderr(&export));
    assert!(
        support::stdout(&export).contains("ferrosift.repro.v1"),
        "{}",
        support::stdout(&export)
    );
    assert!(case_dir.join("manifest.json").exists());
    assert_eq!(fs::read(case_dir.join("input.bin")).expect("input"), b"Hi");

    let check = support::run(
        &["repro", "check", "--case", support::path_text(&case_dir)],
        b"",
    );
    assert!(check.status.success(), "{}", support::stderr(&check));
    assert_eq!(support::stdout(&check).trim(), "passed");
}

#[test]
fn repro_check_fails_when_expected_output_is_tampered() {
    let directory = support::TempDir::new("repro-tamper");
    let recipe = directory.write("recipe.json", r#"[{"op":"To Hex","args":["Space",0]}]"#);
    let input = directory.write("input.bin", b"Hi");
    let case_dir = directory.path("payload-case");

    let export = support::run(
        &[
            "repro",
            "export",
            "--format",
            "cyberchef-v11.3",
            "--input-kind",
            "bytes",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            support::path_text(&input),
            "--out-dir",
            support::path_text(&case_dir),
        ],
        b"",
    );
    assert!(export.status.success(), "{}", support::stderr(&export));

    // Tamper the expected file without updating the manifest digest.
    let expected_path = case_dir.join("expected.json");
    let mut expected: serde_json::Value =
        serde_json::from_slice(&fs::read(&expected_path).expect("expected"))
            .expect("json");
    expected["value"]["text"] = serde_json::json!("tampered");
    fs::write(
        &expected_path,
        serde_json::to_vec_pretty(&expected).expect("serialize"),
    )
    .expect("tamper");

    let check = support::run(
        &["repro", "check", "--case", support::path_text(&case_dir)],
        b"",
    );
    assert!(!check.status.success());
    assert!(
        support::stderr(&check).contains("host.repro.expected_digest_mismatch"),
        "{}",
        support::stderr(&check)
    );
}
