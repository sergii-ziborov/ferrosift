//! Bounded execution through the real CLI process.

use std::fs;

mod support;

const NATIVE_BASE64_ROUND_TRIP: &str = r#"{
  "schema_version": 1,
  "steps": [
    {
      "id": "encode",
      "operation": "encoding.base64.encode@1",
      "arguments": {},
      "disabled": false,
      "breakpoint": false
    },
    {
      "id": "decode",
      "operation": "encoding.base64.decode@1",
      "arguments": {},
      "disabled": false,
      "breakpoint": false
    }
  ],
  "metadata": {}
}"#;

#[test]
fn run_transforms_standard_input_to_raw_standard_output() {
    let directory = support::TempDir::new("run-stdio");
    let recipe = directory.write("recipe.json", r#"[{"op":"To Hex","args":["Space",0]}]"#);
    let output = support::run(
        &[
            "run",
            "--format",
            "cyberchef-v11.3",
            "--input-kind",
            "bytes",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            "-",
        ],
        b"Ferro",
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(output.stdout, b"46 65 72 72 6f");
    assert!(output.stderr.is_empty());
}

#[test]
fn run_writes_a_completed_native_recipe_to_the_requested_file() {
    let directory = support::TempDir::new("run-file");
    let recipe = directory.write("recipe.json", NATIVE_BASE64_ROUND_TRIP);
    let input = directory.write("input.bin", [0x00, 0x46, 0xff, 0x10]);
    let result = directory.path("result.bin");
    let output = support::run(
        &[
            "run",
            "--format",
            "ferrosift",
            "--input-kind",
            "bytes",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            support::path_text(&input),
            "--output",
            support::path_text(&result),
        ],
        b"",
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(result).expect("result must exist"),
        [0x00, 0x46, 0xff, 0x10]
    );
}

#[test]
fn run_rejects_ambiguous_stdin_and_invalid_utf8() {
    let output = support::run(
        &[
            "run",
            "--format",
            "ferrosift",
            "--input-kind",
            "bytes",
            "--recipe",
            "-",
            "--input",
            "-",
        ],
        b"",
    );
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.io.stdin_conflict"),
        "{}",
        support::stderr(&output)
    );

    let directory = support::TempDir::new("run-utf8");
    let recipe = directory.write(
        "recipe.json",
        r#"[{"op":"From Base64","args":["A-Za-z0-9+/=",true,false]}]"#,
    );
    let output = support::run(
        &[
            "run",
            "--format",
            "cyberchef-v11.3",
            "--input-kind",
            "text",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            "-",
        ],
        &[0xff],
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        support::stderr(&output).contains("cli.input.invalid_utf8"),
        "{}",
        support::stderr(&output)
    );
}

#[test]
fn run_does_not_create_output_when_a_breakpoint_pauses_execution() {
    let directory = support::TempDir::new("run-pause");
    let recipe = directory.write(
        "recipe.json",
        r#"{
          "schema_version": 1,
          "steps": [{
            "id": "pause",
            "operation": "encoding.hex.encode@1",
            "arguments": {},
            "disabled": false,
            "breakpoint": true
          }],
          "metadata": {}
        }"#,
    );
    let result = directory.path("must-not-exist.bin");
    let output = support::run(
        &[
            "run",
            "--format",
            "ferrosift",
            "--input-kind",
            "bytes",
            "--recipe",
            support::path_text(&recipe),
            "--input",
            "-",
            "--output",
            support::path_text(&result),
        ],
        b"data",
    );

    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.execution.paused"),
        "{}",
        support::stderr(&output)
    );
    assert!(!result.exists());
}
