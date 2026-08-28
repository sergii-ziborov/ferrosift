//! Byte-for-byte conformance against pinned external reference observations.

use ferrosift_model::CompatibilityProfile;

#[path = "support/differential/mod.rs"]
mod differential;
mod support;

#[test]
fn reference_recipes_match_outputs_and_stopping_positions() {
    let suite = differential::load_suite();
    assert_eq!(suite.reference.name, "CyberChef");
    assert_eq!(suite.reference.version, "11.3.0");
    assert_eq!(
        suite.reference.commit,
        "d24ba1afce2e3a080308b5df7db033332fe94a1a"
    );
    assert_eq!(suite.cases.len(), 79);

    for case in &suite.cases {
        differential::assert_supported_case(CompatibilityProfile::CyberChefV11_3, case);
    }
}

#[test]
fn unsupported_operation_has_a_stable_explicit_finding() {
    let suite = differential::load_suite();
    differential::assert_unsupported_case(CompatibilityProfile::CyberChefV11_3, &suite.unsupported);
}
