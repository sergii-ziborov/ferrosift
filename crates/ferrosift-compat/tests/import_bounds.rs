//! Hard import-boundary and source-preservation tests.

use ferrosift_compat::cyberchef::{
    ImportError, MAX_RECIPE_BYTES, MAX_RECIPE_STEPS, export_source, import_recipe,
};
use ferrosift_core::OperationRegistry;
use serde_json::Value as JsonValue;

#[test]
fn malformed_and_non_array_json_return_distinct_stable_errors() {
    let registry = OperationRegistry::new();

    let malformed = import_recipe(b"[{", &registry).expect_err("JSON is incomplete");
    let non_array =
        import_recipe(br#"{"op":"From Hex"}"#, &registry).expect_err("top level must be an array");

    assert_eq!(malformed, ImportError::MalformedJson);
    assert_eq!(malformed.code(), "compat.cyberchef.malformed_json");
    assert_eq!(non_array, ImportError::ExpectedArray);
    assert_eq!(non_array.code(), "compat.cyberchef.expected_array");
}

#[test]
fn source_byte_limit_is_inclusive_and_checked_before_parsing() {
    let registry = OperationRegistry::new();
    let mut at_limit = vec![b' '; MAX_RECIPE_BYTES];
    at_limit[0] = b'[';
    at_limit[1] = b']';

    let report = import_recipe(&at_limit, &registry).expect("exact byte ceiling is accepted");
    let over_limit = vec![b' '; MAX_RECIPE_BYTES + 1];
    let error = import_recipe(&over_limit, &registry).expect_err("over ceiling is rejected first");

    assert!(report.source.steps().is_empty());
    assert_eq!(error, ImportError::SourceTooLarge);
}

#[test]
fn step_limit_is_inclusive() {
    let registry = OperationRegistry::new();
    let at_limit = json_array_with_nulls(MAX_RECIPE_STEPS);
    let over_limit = json_array_with_nulls(MAX_RECIPE_STEPS + 1);

    let report = import_recipe(at_limit.as_bytes(), &registry).expect("4096 steps are accepted");
    let error = import_recipe(over_limit.as_bytes(), &registry).expect_err("4097 are rejected");

    assert_eq!(report.source.steps().len(), MAX_RECIPE_STEPS);
    assert_eq!(error, ImportError::TooManySteps);
}

#[test]
fn step_ceiling_stops_before_materializing_trailing_source() {
    let registry = OperationRegistry::new();
    let mut source = json_array_with_nulls(MAX_RECIPE_STEPS + 1);
    source.pop();
    source.push_str(",{\"unterminated\":]");
    assert!(source.len() < MAX_RECIPE_BYTES);

    let error = import_recipe(source.as_bytes(), &registry)
        .expect_err("the 4097th complete value crosses the step ceiling");

    assert_eq!(error, ImportError::TooManySteps);
}

#[test]
fn source_export_preserves_every_semantic_json_value() {
    let registry = OperationRegistry::new();
    let input = include_bytes!("fixtures/cyberchef-v11.3.0/recipes.json");
    let report = import_recipe(input, &registry).expect("bounded JSON source is preservable");

    let exported = export_source(&report.source).expect("JSON values serialize");
    let original: JsonValue = serde_json::from_slice(input).expect("fixture is valid JSON");
    let round_trip: JsonValue = serde_json::from_str(&exported).expect("export is valid JSON");

    assert_eq!(round_trip, original);
}

#[test]
fn source_export_preserves_arbitrary_precision_number_tokens() {
    let registry = OperationRegistry::new();
    let input =
        br#"[{"op":"Unknown","args":[184467440737095516160,0.123456789012345678901234567890]}]"#;

    let report = import_recipe(input, &registry).expect("bounded source is preservable");
    let exported = export_source(&report.source).expect("source serializes");

    assert!(exported.contains("184467440737095516160"));
    assert!(exported.contains("0.123456789012345678901234567890"));
}

fn json_array_with_nulls(count: usize) -> String {
    let mut json = String::from("[");
    for index in 0..count {
        if index > 0 {
            json.push(',');
        }
        json.push_str("null");
    }
    json.push(']');
    json
}
