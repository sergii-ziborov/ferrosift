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

    for (format, recipe) in [
        ("ferrosift", &native),
        ("cyberchef-v11.3", &cyberchef),
        ("cyberchef-v11.4", &cyberchef),
    ] {
        let output = support::run(
            &[
                "validate",
                "--format",
                format,
                "--input-kind",
                "bytes",
                "--recipe",
                support::path_text(recipe),
            ],
            b"",
        );
        assert!(output.status.success(), "{}", support::stderr(&output));
        assert_eq!(support::stdout(&output), "valid\n");
        assert!(output.stderr.is_empty());
    }
}

/// The two `CyberChef` formats read the same bytes and differ only in names.
///
/// The reference's recipe model is unchanged between 11.3 and 11.4 — the
/// `Recipe`, `Operation`, `Dish` and `Utils` modules are byte-identical across
/// the two checkouts — so one file loading under both is the correct outcome,
/// not an accident of the flag being ignored. What the flag decides is which
/// names resolve, which the compatibility crate's own tests hold against a
/// fixture that exists in exactly one profile.
#[test]
fn both_cyberchef_formats_run_the_same_recipe() {
    let directory = support::TempDir::new("run-v11-4");
    let recipe = directory.write("cyberchef.json", r#"[{"op":"To Hex","args":["Space",0]}]"#);

    for format in ["cyberchef-v11.3", "cyberchef-v11.4"] {
        let output = support::run(
            &[
                "run",
                "--format",
                format,
                "--input-kind",
                "bytes",
                "--recipe",
                support::path_text(&recipe),
                "--input",
                "-",
            ],
            b"\x00\xff",
        );
        assert!(output.status.success(), "{}", support::stderr(&output));
        assert_eq!(support::stdout(&output), "00 ff");
    }
}

#[test]
fn validate_reports_an_unsupported_operation() {
    let directory = support::TempDir::new("validate-incompatible");
    let unsupported = directory.write("unsupported.json", r#"[{"op":"Magic","args":[]}]"#);

    let output = validate("cyberchef-v11.3", "bytes", &unsupported);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        support::stderr(&output).contains("compat.cyberchef.unknown_operation"),
        "{}",
        support::stderr(&output)
    );
}

/// Text into a byte-reading operation is no longer a mismatch.
///
/// It used to be, and this test used to assert it. The reference carries one
/// value between steps and presents it as whatever the next step asks for, so
/// refusing here rejected recipes that work there — `To Hex` over text is an
/// ordinary thing to want. Both representations are now declared and the
/// conversion is the reference's own.
///
/// Which leaves the two kinds this command offers with nothing to mismatch
/// against, so there is no negative case left to write here. A representation
/// the executor really does refuse — structured values, file lists — cannot be
/// supplied from the command line at all.
#[test]
fn validate_accepts_text_for_a_byte_reading_recipe() {
    let directory = support::TempDir::new("validate-crosskind");
    let recipe = directory.write("to-hex.json", NATIVE_TO_HEX);

    let output = validate("ferrosift", "text", &recipe);
    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout(&output), "valid\n");
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
