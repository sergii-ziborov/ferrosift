//! Conformance against pinned `CyberChef` v11.4.0.
//!
//! A compatibility claim is against a version, not against a project. Adding
//! `11.4` does not retire `11.3`: a caller pinned to the older reference is
//! entitled to know `FerroSift` still matches it, so `corpus.rs` keeps
//! replaying `11.3` and this file replays `11.4` alongside it.
//!
//! `11.4` is stored as a delta against `11.3` (`*.overlay.json`) rather than
//! as a second copy of the fixtures. That is a storage decision and not an
//! evidential one: [`differential::apply_overlay`] reconstructs `11.4`'s own
//! recorded bytes, and every case below is replayed against those. Where the
//! two references agree the reconstructed byte string is `11.3`'s because
//! `11.4` produced exactly it — which the oracle checked case by case, not by
//! assumption.

use std::collections::{BTreeMap, BTreeSet};

use ferrosift_model::CompatibilityProfile;

#[path = "support/differential/mod.rs"]
mod differential;
mod support;

const EXEMPTIONS_JSON: &str = include_str!("../../../docs/compatibility/exemptions.json");

const BASELINE: (&str, &str) = ("11.3.0", "d24ba1afce2e3a080308b5df7db033332fe94a1a");
const COMPARED: (&str, &str) = ("11.4.0", "49d1a5634a67a3b806c6db0fdca7dcecb41a776c");

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

/// The 11.4 corpus, rebuilt from the baseline and the recorded delta.
fn corpus_11_4() -> Vec<differential::Case> {
    let overlay = differential::load_corpus_overlay_11_4();
    assert_eq!(overlay.reference.version, COMPARED.0);
    assert_eq!(overlay.reference.commit, COMPARED.1);
    assert_eq!(overlay.baseline.version, BASELINE.0);
    assert_eq!(
        overlay.baseline.commit, BASELINE.1,
        "the overlay was built against a different baseline than this test replays"
    );
    differential::apply_overlay(&differential::load_corpus().cases, &overlay)
}

#[test]
fn corpus_matches_reference_bytes_at_every_prefix() {
    let cases = corpus_11_4();
    assert!(
        cases.len() >= 500,
        "corpus must stay large; found {} cases",
        cases.len()
    );
    for case in &cases {
        differential::assert_supported_case(case);
    }
}

#[test]
fn differential_suite_matches_reference_bytes_at_every_prefix() {
    let overlay = differential::load_suite_overlay_11_4();
    assert_eq!(overlay.reference.version, COMPARED.0);
    assert_eq!(overlay.baseline.commit, BASELINE.1);
    for case in &differential::apply_overlay(&differential::load_suite().cases, &overlay) {
        differential::assert_supported_case(case);
    }
}

/// Every 11.4 alias must be backed by 11.4 evidence, on the same terms as 11.3.
///
/// The alias is the claim "this operation is the reference's operation of that
/// name". Copying the 11.3 alias list forward would assert that for 11.4
/// without having checked it, which is the failure this file exists to
/// prevent — so the claim is only allowed where a replayed 11.4 case backs it
/// or a documented exemption says why not.
#[test]
fn every_cyberchef_11_4_alias_is_covered_or_explicitly_exempt() {
    let mut coverage: BTreeMap<String, usize> = BTreeMap::new();
    for case in &corpus_11_4() {
        for operation in case.operations() {
            *coverage.entry(operation.to_string()).or_default() += 1;
        }
    }

    let exemptions = corpus_exemptions();
    let registry = support::registry();
    let mut aliases: BTreeSet<String> = BTreeSet::new();
    for specification in registry.catalog() {
        for alias in &specification.aliases {
            if alias.profile == CompatibilityProfile::CyberChefV11_4 {
                aliases.insert(alias.name.clone());
            }
        }
    }

    assert!(
        !aliases.is_empty(),
        "no operation claims a CyberChef 11.4 alias, so this gate proves nothing"
    );

    for alias in &aliases {
        let covered = coverage.contains_key(alias.as_str());
        let exempt = exemptions.contains(alias.as_str());
        assert!(
            covered || exempt,
            "CyberChef 11.4 alias `{alias}` has no corpus coverage and no documented exemption"
        );
        assert!(
            !(covered && exempt),
            "CyberChef 11.4 alias `{alias}` is both covered and exempt; drop the stale exemption"
        );
    }
}

/// An operation may arrive between profiles, but it may not be renamed across
/// them, and it may not silently disappear from the newer one.
///
/// Upstream renaming an operation would not change any output byte, so the
/// replays above would all pass while every recipe naming the old name had
/// quietly stopped working. What rules that out is that the oracle baked this
/// corpus *through* 11.4 using these exact names: a rename would have failed
/// the bake and dropped the case, so a name's presence in the 11.4 corpus is
/// the evidence that 11.4 still answers to it. This turns that into a rule the
/// catalog has to keep — one spec never carries two names.
///
/// Claiming 11.4 without 11.3 is the one asymmetry allowed, and it is a fact
/// about the reference rather than about this port: 11.4 introduced operations
/// 11.3 does not have, and asserting those names in 11.3 would assert something
/// the older reference cannot answer to. It costs nothing in evidence —
/// `every_cyberchef_11_4_alias_is_covered_or_explicitly_exempt` still demands a
/// replayed 11.4 case for the alias that is claimed.
///
/// The reverse — 11.3 without 11.4 — would mean upstream *removed* an
/// operation, which is a different claim needing its own evidence and which
/// `build_since` cannot currently express. It is refused here rather than left
/// to be discovered later.
#[test]
fn an_alias_keeps_one_name_across_the_profiles_that_have_it() {
    let registry = support::registry();
    let mut both = 0usize;
    for specification in registry.catalog() {
        let named = |profile| {
            specification
                .aliases
                .iter()
                .find(|alias| alias.profile == profile)
                .map(|alias| alias.name.clone())
        };
        let old = named(CompatibilityProfile::CyberChefV11_3);
        let new = named(CompatibilityProfile::CyberChefV11_4);
        let identifier = specification.id.as_str();

        match (&old, &new) {
            (Some(_), Some(_)) => {
                both += 1;
                assert_eq!(
                    old, new,
                    "`{identifier}` claims different names in 11.3 and 11.4; a genuine \
                     rename needs two specs and a versioned identifier, not one spec \
                     with two names"
                );
            }
            (Some(name), None) => panic!(
                "`{identifier}` claims `{name}` in 11.3 and nothing in 11.4, which says \
                 upstream removed it; that needs its own evidence rather than a gap in \
                 the alias list"
            ),
            // Introduced in 11.4, or native and aliased in neither.
            (None, _) => {}
        }
    }
    assert!(
        both > 100,
        "expected the bulk of the catalog to claim both profiles; found {both}"
    );
}
