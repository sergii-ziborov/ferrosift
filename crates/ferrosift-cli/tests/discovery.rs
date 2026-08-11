//! Operation discovery through the real CLI process.

mod support;

#[test]
fn operations_lists_canonical_ids_in_stable_order() {
    let output = support::run(&["operations"], b"");

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(
        support::stdout(&output),
        concat!(
            "core.identity@1\n",
            "encoding.base64.decode@1\n",
            "encoding.base64.encode@1\n",
            "encoding.hex.decode@1\n",
            "encoding.hex.encode@1\n",
        )
    );
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
