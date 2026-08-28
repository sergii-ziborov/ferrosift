//! Which reference version a recipe is read and written as.
//!
//! The recipe *format* is identical across the `CyberChef` releases this
//! speaks — the reference's `Recipe`, `Operation`, `Dish` and `Utils` modules
//! are byte-identical between 11.3 and 11.4. What differs is the set of names
//! that exist. These tests hold that distinction: one parser, and a profile
//! that decides whether a name resolves at all.
//!
//! The fixtures use a synthetic operation rather than a real one on purpose. A
//! real 11.4-only operation would make these tests pass for a second reason —
//! that this port happens to have that operation — and they would stop failing
//! if the profile argument were ignored. A fixture that exists in exactly one
//! profile can only pass if the profile is honoured.

mod support;

use ferrosift_compat::cyberchef::{ExportError, ImportError, export_recipe, import_recipe};
use ferrosift_model::{
    ArgumentKind, CompatibilityAlias, CompatibilityProfile, Recipe, RecipeMetadata,
};

use support::{argument, registry_of};

/// An operation the reference introduced in 11.4, plus one it always had.
fn split_registry() -> ferrosift_core::OperationRegistry {
    registry_of(&[
        (
            "test.introduced@1",
            &[(CompatibilityProfile::CyberChefV11_4, "Introduced Later")],
            vec![argument("value", ArgumentKind::Integer, true, None)],
        ),
        (
            "test.always@1",
            &[
                (CompatibilityProfile::CyberChefV11_3, "Always Present"),
                (CompatibilityProfile::CyberChefV11_4, "Always Present"),
            ],
            vec![],
        ),
    ])
}

#[test]
fn name_introduced_in_11_4_resolves_only_under_11_4() {
    let registry = split_registry();
    let source = br#"[{"op":"Introduced Later","args":[7]}]"#;

    let later = import_recipe(source, CompatibilityProfile::CyberChefV11_4, &registry)
        .expect("bounded recipe parses");
    let recipe = later.recipe.expect("11.4 knows the name");
    assert_eq!(recipe.steps[0].operation.as_str(), "test.introduced@1");
    assert!(later.findings.is_empty());

    let earlier = import_recipe(source, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("bounded recipe parses");
    assert!(earlier.recipe.is_none());
    let finding = &earlier.findings[0];
    assert_eq!(finding.code, "compat.cyberchef.unknown_operation");
    assert_eq!(
        finding.original_operation.as_deref(),
        Some("Introduced Later")
    );
    // The explanation names the version that was asked, because the answer
    // depends on it: this is 11.3 not having the operation, not FerroSift.
    assert_eq!(
        finding.explanation,
        "operation has no exact CyberChef 11.3 alias"
    );
}

#[test]
fn name_present_in_both_profiles_resolves_in_both() {
    let registry = split_registry();
    let source = br#"[{"op":"Always Present","args":[]}]"#;

    for profile in [
        CompatibilityProfile::CyberChefV11_3,
        CompatibilityProfile::CyberChefV11_4,
    ] {
        let report = import_recipe(source, profile, &registry).expect("bounded recipe parses");
        let recipe = report
            .recipe
            .unwrap_or_else(|| panic!("{profile:?} knows the name"));
        assert_eq!(recipe.steps[0].operation.as_str(), "test.always@1");
    }
}

#[test]
fn export_refuses_a_profile_that_has_no_name_for_the_operation() {
    let registry = split_registry();
    let source = br#"[{"op":"Introduced Later","args":[7]}]"#;
    let recipe = import_recipe(source, CompatibilityProfile::CyberChefV11_4, &registry)
        .expect("bounded recipe parses")
        .recipe
        .expect("11.4 knows the name");

    // Writing it as 11.3 would produce a recipe the older reference refuses to
    // load. Refusing here is the honest answer: there is no name to write.
    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_3, &registry),
        Err(ExportError::MissingAlias)
    );
    assert_eq!(
        export_recipe(&recipe, CompatibilityProfile::CyberChefV11_4, &registry)
            .expect("11.4 has a name"),
        r#"[{"op":"Introduced Later","args":[7]}]"#
    );
}

#[test]
fn eleven_four_import_and_export_round_trip_byte_for_byte() {
    let registry = split_registry();
    let source = br#"[{"op":"Introduced Later","args":[7]},{"op":"Always Present","args":[]}]"#;

    let recipe = import_recipe(source, CompatibilityProfile::CyberChefV11_4, &registry)
        .expect("bounded recipe parses")
        .recipe
        .expect("both names exist in 11.4");
    let exported = export_recipe(&recipe, CompatibilityProfile::CyberChefV11_4, &registry)
        .expect("every step has an 11.4 name");

    assert_eq!(exported.as_bytes(), source);
}

#[test]
fn a_name_may_be_spelled_differently_by_each_profile() {
    // Nothing forces the two profiles to agree on spelling. If upstream ever
    // renames an operation, the alias list is where that is recorded, and both
    // names have to keep resolving to the same canonical operation.
    let registry = registry_of(&[(
        "test.renamed@1",
        &[
            (CompatibilityProfile::CyberChefV11_3, "Old Name"),
            (CompatibilityProfile::CyberChefV11_4, "New Name"),
        ],
        vec![],
    )]);

    for (profile, name) in [
        (CompatibilityProfile::CyberChefV11_3, "Old Name"),
        (CompatibilityProfile::CyberChefV11_4, "New Name"),
    ] {
        let source = format!(r#"[{{"op":"{name}","args":[]}}]"#);
        let recipe = import_recipe(source.as_bytes(), profile, &registry)
            .expect("bounded recipe parses")
            .recipe
            .unwrap_or_else(|| panic!("{profile:?} knows {name}"));
        assert_eq!(recipe.steps[0].operation.as_str(), "test.renamed@1");
        assert_eq!(
            export_recipe(&recipe, profile, &registry).expect("the profile has a name"),
            source
        );
    }
}

#[test]
fn the_native_profile_is_not_a_cyberchef_interchange_format() {
    // Native is a naming profile for FerroSift's own catalog, not a serialized
    // recipe dialect. Asking for it here is a caller mistake with its own
    // code, rather than something that half-works.
    let registry = split_registry();
    assert_eq!(
        import_recipe(
            br#"[{"op":"Always Present","args":[]}]"#,
            CompatibilityProfile::Native,
            &registry,
        ),
        Err(ImportError::UnsupportedProfile)
    );
    assert_eq!(
        export_recipe(&empty_recipe(), CompatibilityProfile::Native, &registry),
        Err(ExportError::UnsupportedProfile)
    );
}

#[test]
fn cyberchef_since_claims_the_named_version_and_every_later_one() {
    // The alias helper is what keeps a specification from having to re-declare
    // a name once per reference release. An operation that has always existed
    // is claimed in both profiles; one introduced in 11.4 is claimed only from
    // there. Adding a future profile extends the first set and not the second.
    let always = CompatibilityAlias::cyberchef_since(CompatibilityProfile::CyberChefV11_3, "A");
    assert_eq!(
        always.iter().map(|alias| alias.profile).collect::<Vec<_>>(),
        vec![
            CompatibilityProfile::CyberChefV11_3,
            CompatibilityProfile::CyberChefV11_4,
        ]
    );

    let later = CompatibilityAlias::cyberchef_since(CompatibilityProfile::CyberChefV11_4, "B");
    assert_eq!(
        later.iter().map(|alias| alias.profile).collect::<Vec<_>>(),
        vec![CompatibilityProfile::CyberChefV11_4]
    );

    assert!(CompatibilityAlias::cyberchef_since(CompatibilityProfile::Native, "C").is_empty());
}

fn empty_recipe() -> Recipe {
    Recipe {
        schema_version: ferrosift_model::SchemaVersion::CURRENT.get(),
        metadata: RecipeMetadata::default(),
        steps: Vec::new(),
    }
}
