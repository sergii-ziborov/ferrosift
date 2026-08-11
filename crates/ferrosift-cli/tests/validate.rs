//! Recipe validation through the real CLI process.

mod support;

const NATIVE_TO_HEX: &str = r#"{
  "schema_version": 1,
  "steps": [{
    "id": "hex",
    "operation": "encoding.hex.encode@1",
    "arguments": {},
    "disabled": false,
    "breakpoint": false
  }],
  "metadata": {}
}"#;

#[test]
fn validate_accepts_native_and_cyberchef_recipes_after_full_preflight() {
    let directory = support::TempDir::new("validate-supported");
    let native = directory.write("native.json", NATIVE_TO_HEX);
    let cyberchef = directory.write(
        "cyberchef.json",
        r#"[{"op":"To Base64","args":["A-Za-z0-9+/="]}]"#,
    );

    for (format, recipe) in [("ferrosift", native), ("cyberchef-v11.3", cyberchef)] {
        let output = support::run(
            &[
                "validate",
                "--format",
                format,
                "--input-kind",
                "bytes",
                "--recipe",
                support::path_text(&recipe),
            ],
            b"",
        );
        assert!(output.status.success(), "{}", support::stderr(&output));
        assert_eq!(support::stdout(&output), "valid\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn validate_reports_unsupported_operations_and_input_kinds() {
    let directory = support::TempDir::new("validate-incompatible");
    let unsupported = directory.write(
        "unsupported.json",
        r#"[{"op":"URL Encode","args":[false]}]"#,
    );
    let mismatch = directory.write("mismatch.json", NATIVE_TO_HEX);

    let output = validate("cyberchef-v11.3", "bytes", &unsupported);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        support::stderr(&output).contains("compat.cyberchef.unknown_operation"),
        "{}",
        support::stderr(&output)
    );

    let output = validate("ferrosift", "text", &mismatch);
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("core.executor.input_kind_mismatch"),
        "{}",
        support::stderr(&output)
    );
}

#[test]
fn validate_rejects_unknown_schema_and_malformed_json() {
    let directory = support::TempDir::new("validate-malformed");
    let schema = directory.write(
        "schema.json",
        NATIVE_TO_HEX.replace("\"schema_version\": 1", "\"schema_version\": 2"),
    );
    let malformed = directory.write("malformed.json", b"{");

    let output = validate("ferrosift", "bytes", &schema);
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.recipe.schema_unsupported"),
        "{}",
        support::stderr(&output)
    );

    let output = validate("ferrosift", "bytes", &malformed);
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.recipe.malformed"),
        "{}",
        support::stderr(&output)
    );
}

#[test]
fn validate_enforces_an_inclusive_recipe_byte_ceiling() {
    let at_limit = vec![b' '; 1_048_576];
    let output = support::run(
        &[
            "validate",
            "--format",
            "ferrosift",
            "--input-kind",
            "bytes",
            "--recipe",
            "-",
        ],
        &at_limit,
    );
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.recipe.malformed"),
        "{}",
        support::stderr(&output)
    );

    let above_limit = vec![b' '; 1_048_577];
    let output = support::run(
        &[
            "validate",
            "--format",
            "ferrosift",
            "--input-kind",
            "bytes",
            "--recipe",
            "-",
        ],
        &above_limit,
    );
    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("cli.recipe.too_large"),
        "{}",
        support::stderr(&output)
    );
}

fn validate(format: &str, input_kind: &str, recipe: &std::path::Path) -> std::process::Output {
    support::run(
        &[
            "validate",
            "--format",
            format,
            "--input-kind",
            input_kind,
            "--recipe",
            support::path_text(recipe),
        ],
        b"",
    )
}
