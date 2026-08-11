//! Public API tests for `FerroSift` schema versions.

use ferrosift_model::SchemaVersion;

#[test]
fn current_schema_version_is_one() {
    assert_eq!(SchemaVersion::CURRENT.get(), 1);
}

#[test]
fn schema_version_preserves_an_explicit_value() {
    assert_eq!(SchemaVersion::new(7).get(), 7);
}

#[test]
fn schema_version_display_is_stable_decimal() {
    assert_eq!(SchemaVersion::new(12).to_string(), "12");
}
