//! Differential replay of flow control against pinned `CyberChef` v11.3.0.
//!
//! Separate from `corpus.rs` because the fixture is produced a different way.
//! The reference's Node API refuses these operations — "flowControl operations
//! like Return are not currently allowed in recipes for chef.bake" — and does
//! not export Label or Comment as functions at all. That is the wrapper's
//! restriction, so the generator drives `Recipe.execute` directly instead: the
//! same code path the browser uses, from the same pinned commit.
//!
//! What is compared is what the rest of the corpus compares: the exact output
//! bytes at every recipe prefix. The step *count* is deliberately not compared
//! — a recipe that loops runs a step more than once and one that returns runs
//! some not at all, and the reference reports no count to compare against.

use ferrosift_model::CompatibilityProfile;

#[path = "support/differential/mod.rs"]
mod differential;
mod support;

/// Every operation the fixture is there to pin.
///
/// Named rather than counted: a fixture that lost its Subsection cases in a
/// regeneration would still have plenty of cases, and this is what notices.
const PINNED: &[&str] = &[
    "Comment",
    "Conditional Jump",
    "Fork",
    "Jump",
    "Label",
    "Merge",
    "Return",
    "Subsection",
];

#[test]
fn flow_control_matches_reference_bytes_at_every_prefix() {
    let suite = differential::load_flow();
    assert_eq!(suite.reference.name, "CyberChef");
    assert_eq!(suite.reference.version, "11.3.0");
    assert_eq!(
        suite.reference.commit,
        "d24ba1afce2e3a080308b5df7db033332fe94a1a"
    );

    for case in &suite.cases {
        differential::assert_flow_case(CompatibilityProfile::CyberChefV11_3, case);
    }
}

#[test]
fn every_flow_control_operation_is_exercised() {
    let suite = differential::load_flow();
    for operation in PINNED {
        let uses = suite
            .cases
            .iter()
            .filter(|case| case.operations().contains(operation))
            .count();
        assert!(
            uses >= 2,
            "flow fixture exercises `{operation}` in {uses} case(s); \
             regenerate it with `cargo xtask cyberchef generate`"
        );
    }
}
