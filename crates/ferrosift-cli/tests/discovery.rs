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
