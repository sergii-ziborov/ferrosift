//! Conformance for Suggest recipe (Magic-as-advisor).

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn suggest_args(depth: i128, max_results: i128, intensive: bool, crib: &str) -> Arguments {
    Arguments::from([
        ("depth".into(), ArgumentValue::Integer(depth)),
        ("max_results".into(), ArgumentValue::Integer(max_results)),
        ("intensive".into(), ArgumentValue::Boolean(intensive)),
        ("crib".into(), ArgumentValue::Text(crib.into())),
    ])
}

#[test]
fn suggest_ranks_from_hex_for_hex_payload() {
    // "FerroSift" as hex
    let report = support::output_text(support::run(
        "analysis.suggest@1",
        suggest_args(1, 8, false, ""),
        support::text("466572726f53696674"),
    ));
    assert!(report.starts_with("FerroSift Suggest recipe\n"), "{report}");
    assert!(report.contains("From Hex"), "{report}");
    assert!(report.contains("encoding.hex.decode@1"), "{report}");
    assert!(report.contains("preview: FerroSift"), "{report}");
    assert!(
        report.contains(r#"recipe: [{"op":"From Hex","args":["Auto"]}]"#),
        "{report}"
    );
    // Magic alias is intentionally not registered.
    assert!(
        support::registry()
            .resolve_alias(
                ferrosift_model::CompatibilityProfile::CyberChefV11_3,
                "Magic"
            )
            .is_none()
    );
}

#[test]
fn suggest_ranks_from_base64() {
    // echo -n FerroSift | base64
    let report = support::output_text(support::run(
        "analysis.suggest@1",
        suggest_args(1, 8, false, ""),
        support::text("RmVycm9TaWZ0"),
    ));
    assert!(report.contains("From Base64"), "{report}");
    assert!(report.contains("preview: FerroSift"), "{report}");
}

#[test]
fn suggest_detects_gzip_magic_on_bytes() {
    // gzip of "hi" - minimal valid-ish probe: run Gzip then Suggest on bytes
    let compressed = support::output_bytes(support::run(
        "compression.gzip@1",
        Arguments::from([
            (
                "compression_type".into(),
                ArgumentValue::Text("Dynamic Huffman Coding".into()),
            ),
            ("filename".into(), ArgumentValue::Text(String::new())),
            ("comment".into(), ArgumentValue::Text(String::new())),
            (
                "include_file_checksum".into(),
                ArgumentValue::Boolean(false),
            ),
        ]),
        Value::Bytes(b"hello suggest".to_vec()),
    ));
    assert!(compressed.starts_with(&[0x1f, 0x8b]));
    let report = support::output_text(support::run(
        "analysis.suggest@1",
        suggest_args(1, 8, false, ""),
        Value::Bytes(compressed),
    ));
    assert!(report.contains("Gunzip"), "{report}");
    assert!(report.contains("compression.gunzip@1"), "{report}");
}

#[test]
fn suggest_crib_filters_previews() {
    let report = support::output_text(support::run(
        "analysis.suggest@1",
        suggest_args(1, 8, false, "no-such-crib-token"),
        support::text("466572726f53696674"),
    ));
    assert!(report.contains("(no suggestions)"), "{report}");
}

#[test]
fn suggest_depth_can_chain_hex_then_gzip() {
    // Hex of a gzip payload for "chain".
    let compressed = support::output_bytes(support::run(
        "compression.gzip@1",
        Arguments::from([
            (
                "compression_type".into(),
                ArgumentValue::Text("Dynamic Huffman Coding".into()),
            ),
            ("filename".into(), ArgumentValue::Text(String::new())),
            ("comment".into(), ArgumentValue::Text(String::new())),
            (
                "include_file_checksum".into(),
                ArgumentValue::Boolean(false),
            ),
        ]),
        Value::Bytes(b"chain".to_vec()),
    ));
    let mut hex = String::new();
    for byte in &compressed {
        hex.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        hex.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    let report = support::output_text(support::run(
        "analysis.suggest@1",
        suggest_args(2, 16, false, ""),
        support::text(&hex),
    ));
    assert!(report.contains("From Hex"), "{report}");
    // Depth-2 should often surface a gunzip chain after hex decode.
    assert!(
        report.contains("Gunzip") || report.contains("chain: From Hex"),
        "{report}"
    );
}
