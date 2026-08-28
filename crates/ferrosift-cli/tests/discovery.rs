//! Operation discovery through the real CLI process.

mod support;

/// The published compatibility ledger, which is generated from this same
/// catalog. Comparing against it here means the CLI and the ledger cannot
/// disagree, and replaces the frozen literal that used to live in this test —
/// a list that had to be re-typed by hand on every new operation and said
/// nothing about whether the catalog was right.
const LEDGER: &str = include_str!("../../../docs/compatibility/ledger.json");

#[test]
fn operations_lists_every_ledger_id_in_sorted_order() {
    let output = support::run(&["operations"], b"");
    assert!(output.status.success(), "{}", support::stderr(&output));

    let ledger: serde_json::Value = serde_json::from_str(LEDGER).expect("ledger must be JSON");
    let expected: Vec<&str> = ledger["operations"]
        .as_array()
        .expect("ledger must list operations")
        .iter()
        .map(|entry| entry["id"].as_str().expect("every entry must have an id"))
        .collect();

    let listed: Vec<String> = support::stdout(&output)
        .lines()
        .map(alloc_string)
        .collect::<Vec<_>>();

    assert_eq!(listed, expected, "CLI catalog differs from the ledger");
    let mut sorted = listed.clone();
    sorted.sort();
    assert_eq!(listed, sorted, "catalog must be listed in sorted order");
}

fn alloc_string(value: &str) -> String {
    value.to_owned()
}

/// The catalog says once what it stands on, and names every target it claims.
///
/// This block used to be five records on every one of the specifications, where
/// no caller could read it. Published once it answers the question a reviewer
/// actually asks — what backs this? — and the target check is the half that can
/// go wrong silently: an operation may not claim a target the build did not run.
#[test]
fn the_catalog_publishes_the_evidence_it_stands_on() {
    let output = support::run(&["operations", "--format", "json"], b"");
    assert!(output.status.success(), "{}", support::stderr(&output));

    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("catalog must be JSON");
    let evidence = &document["evidence"];
    for dimension in ["provenance", "license", "conformance", "benchmark"] {
        assert_eq!(
            evidence[dimension]["state"], "passed",
            "{dimension} must be backed by something"
        );
        assert!(
            evidence[dimension]["reference"].is_string(),
            "{dimension} must say where to read it"
        );
    }

    let checked = evidence["target_checks"]
        .as_object()
        .expect("the manifest must list the targets this build ran");
    for operation in document["operations"]
        .as_array()
        .expect("catalog must list operations")
    {
        for target in operation["targets"].as_array().expect("declared targets") {
            let name = target.as_str().expect("target names are strings");
            assert!(
                checked.contains_key(name),
                "{} claims target {name}, which the manifest does not cover",
                operation["id"]
            );
        }
    }
}

#[test]
fn describe_writes_the_complete_operation_spec_as_json() {
    let output = support::run(&["describe", "encoding.hex.encode@1"], b"");

    assert!(output.status.success(), "{}", support::stderr(&output));
    let spec: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("description must be JSON");
    assert_eq!(spec["id"], "encoding.hex.encode@1");
    assert_eq!(spec["display_name"], "To Hex");
    assert_eq!(spec["arguments"].as_array().map(Vec::len), Some(2));
}

#[test]
fn describe_rejects_an_unknown_operation_with_a_stable_code() {
    let output = support::run(&["describe", "encoding.missing@1"], b"");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        support::stderr(&output).contains("cli.operation.unknown"),
        "{}",
        support::stderr(&output)
    );
}
