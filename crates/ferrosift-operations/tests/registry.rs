//! The registry's families agree with what its operations say about themselves.
//!
//! Registration is grouped by family, and a family is only useful while it
//! means something. Without a check, a family becomes a junk drawer one
//! convenient placement at a time — which is what happened to the grouping
//! this replaced, where `register_shape` had collected HTTP framing, Braille,
//! `NetBIOS` names, and `PowerSet`.
//!
//! So the family table declares which catalog categories it accepts, and this
//! refuses an operation registered anywhere its own specification disagrees
//! with. Getting it wrong is then a failing test rather than a slow drift.

use std::collections::{BTreeMap, BTreeSet};

#[test]
fn every_operation_sits_in_a_family_that_accepts_its_category() {
    let families = ferrosift_operations::registry_testing::families().expect("families build");
    let mut misplaced = Vec::new();

    for family in &families {
        for (name, category) in &family.registered {
            if !family.categories.contains(&category.as_str()) {
                misplaced.push(format!(
                    "`{name}` declares category {category:?} but is registered in the \
                     `{}` family, which accepts {:?}",
                    family.name, family.categories
                ));
            }
        }
    }

    assert!(
        misplaced.is_empty(),
        "{} operation(s) are registered in a family that does not accept them:\n{}\n\
         Either move the registration, or widen that family's category list and say \
         in the table why the merge is right.",
        misplaced.len(),
        misplaced.join("\n")
    );
}

#[test]
fn the_families_together_are_the_whole_catalog() {
    let families = ferrosift_operations::registry_testing::families().expect("families build");
    let registry = ferrosift_operations::default_registry().expect("registry");

    let mut from_families: BTreeSet<String> = BTreeSet::new();
    for family in &families {
        for (name, _) in &family.registered {
            assert!(
                from_families.insert(name.clone()),
                "`{name}` is registered by more than one family"
            );
        }
    }

    let whole: BTreeSet<String> = registry
        .catalog()
        .map(|spec| spec.display_name.clone())
        .collect();

    let missing: Vec<&String> = whole.difference(&from_families).collect();
    let extra: Vec<&String> = from_families.difference(&whole).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the family table and the default registry disagree.\n\
         In the registry but no family: {missing:?}\n\
         In a family but not the registry: {extra:?}"
    );
}

/// No family may accept a category no operation actually declares.
///
/// A stale entry reads as a promise that the family holds something it does
/// not, and it would silently permit a future operation into the wrong place.
#[test]
fn no_family_claims_a_category_it_does_not_hold() {
    let families = ferrosift_operations::registry_testing::families().expect("families build");

    // Under a reduced feature set a family can legitimately be empty, so the
    // check is against what this build actually contains.
    let present: BTreeSet<&str> = families
        .iter()
        .flat_map(|family| {
            family
                .registered
                .iter()
                .map(|(_, category)| category.as_str())
        })
        .collect();

    let mut stale = Vec::new();
    for family in &families {
        for category in family.categories {
            if present.contains(category)
                && !family.registered.iter().any(|(_, held)| held == category)
            {
                stale.push(format!(
                    "the `{}` family accepts {category:?}, but holds none and another \
                     family does",
                    family.name
                ));
            }
        }
    }

    assert!(stale.is_empty(), "{}", stale.join("\n"));
}

/// Every category an operation declares belongs to exactly one family.
#[test]
fn no_category_is_accepted_by_two_families() {
    let families = ferrosift_operations::registry_testing::families().expect("families build");
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();

    for family in &families {
        for category in family.categories {
            if let Some(previous) = owner.insert(category, family.name) {
                panic!(
                    "category {category:?} is accepted by both `{previous}` and `{}`; \
                     one operation would then have two right homes",
                    family.name
                );
            }
        }
    }
}
