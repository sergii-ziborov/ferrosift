//! Operation discovery through the real CLI process.

mod support;

#[test]
fn operations_lists_canonical_ids_in_stable_order() {
    let output = support::run(&["operations"], b"");

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(
        support::stdout(&output),
        concat!(
            "compression.gunzip@1\n",
            "compression.gzip@1\n",
            "compression.zlib.deflate@1\n",
            "compression.zlib.inflate@1\n",
            "core.identity@1\n",
            "data.drop_bytes@1\n",
            "data.head@1\n",
            "data.take_bytes@1\n",
            "encoding.base32.decode@1\n",
            "encoding.base32.encode@1\n",
            "encoding.base45.decode@1\n",
            "encoding.base45.encode@1\n",
            "encoding.base58.decode@1\n",
            "encoding.base58.encode@1\n",
            "encoding.base64.decode@1\n",
            "encoding.base64.encode@1\n",
            "encoding.base85.decode@1\n",
            "encoding.base85.encode@1\n",
            "encoding.binary.decode@1\n",
            "encoding.binary.encode@1\n",
            "encoding.charcode.decode@1\n",
            "encoding.charcode.encode@1\n",
            "encoding.decimal.decode@1\n",
            "encoding.decimal.encode@1\n",
            "encoding.hex.decode@1\n",
            "encoding.hex.encode@1\n",
            "encoding.hexdump.decode@1\n",
            "encoding.hexdump.encode@1\n",
            "encoding.html.decode@1\n",
            "encoding.html.encode@1\n",
            "encoding.octal.decode@1\n",
            "encoding.octal.encode@1\n",
            "encoding.rot13@1\n",
            "encoding.url.decode@1\n",
            "encoding.url.encode@1\n",
            "hash.hmac@1\n",
            "hash.md5@1\n",
            "hash.sha1@1\n",
            "hash.sha2@1\n",
            "logic.xor@1\n",
            "text.find_replace@1\n",
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
