//! Automatic differential corpus against pinned `CyberChef` v11.3.0.
//!
//! The corpus is machine-generated (see the private oracle) by baking many
//! deterministically sampled inputs through the pinned reference. Each case is
//! replayed here and must match the reference output bytes and stopping
//! position at every recipe prefix. A coverage gate guarantees that every
//! CyberChef-aliased operation is exercised, so no operation is silently
//! unverified.

use std::collections::{BTreeMap, BTreeSet};

use ferrosift_model::CompatibilityProfile;

#[path = "support/differential/mod.rs"]
mod differential;
mod support;

/// Aliased operations deliberately absent from the byte-for-byte corpus, each
/// with a reason that keeps the absence honest rather than silent.
///
/// The list lives in `docs/compatibility/exemptions.json` so that this gate and
/// the published compatibility ledger cannot disagree about what is exempt.
const EXEMPTIONS_JSON: &str = include_str!("../../../docs/compatibility/exemptions.json");

#[derive(serde::Deserialize)]
struct ExemptionFile {
    exemptions: Vec<Exemption>,
}

#[derive(serde::Deserialize)]
struct Exemption {
    alias: String,
}

fn corpus_exemptions() -> BTreeSet<String> {
    let file: ExemptionFile =
        serde_json::from_str(EXEMPTIONS_JSON).expect("exemptions.json must be valid JSON");
    file.exemptions
        .into_iter()
        .map(|exemption| exemption.alias)
        .collect()
}

#[test]
fn corpus_matches_reference_bytes_at_every_prefix() {
    let suite = differential::load_corpus();
    assert_eq!(suite.reference.name, "CyberChef");
    assert_eq!(suite.reference.version, "11.3.0");
    assert_eq!(
        suite.reference.commit,
        "d24ba1afce2e3a080308b5df7db033332fe94a1a"
    );
    assert!(
        suite.cases.len() >= 500,
        "corpus must stay large; found {} cases",
        suite.cases.len()
    );

    for case in &suite.cases {
        differential::assert_supported_case(case);
    }
}

#[test]
fn every_cyberchef_alias_is_covered_or_explicitly_exempt() {
    let suite = differential::load_corpus();
    let mut coverage: BTreeMap<&str, usize> = BTreeMap::new();
    for case in &suite.cases {
        for operation in case.operations() {
            *coverage.entry(operation).or_default() += 1;
        }
    }

    let exemptions = corpus_exemptions();
    let registry = support::registry();
    let mut aliases: BTreeSet<String> = BTreeSet::new();
    for specification in registry.catalog() {
        for alias in &specification.aliases {
            if alias.profile == CompatibilityProfile::CyberChefV11_3 {
                aliases.insert(alias.name.clone());
            }
        }
    }

    for alias in &aliases {
        let covered = coverage.contains_key(alias.as_str());
        let exempt = exemptions.contains(alias.as_str());
        assert!(
            covered || exempt,
            "CyberChef alias `{alias}` has no corpus coverage and no documented exemption"
        );
        assert!(
            !(covered && exempt),
            "CyberChef alias `{alias}` is both covered and exempt; drop the stale exemption"
        );
    }

    for exempt in &exemptions {
        assert!(
            aliases.contains(exempt),
            "exemption `{exempt}` names an operation that is no longer registered"
        );
    }
}
