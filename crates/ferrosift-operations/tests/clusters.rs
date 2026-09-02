//! What the operation identifiers claim about how the catalog is grouped.
//!
//! An id like `encoding.base64.decode@1` says two things: what the operation
//! is, and who its siblings are. The second half was a convention nothing
//! enforced — every couple in the catalog happened to be named consistently
//! because the same person named them, and nothing would have noticed the first
//! one that was not.
//!
//! These are the rules that convention amounts to, written down and checked.
//! One of them found a real defect on the first run: `From Case Insensitive
//! Regex` was the only operation in the catalog whose partner pointed at it
//! while it pointed at nothing.

use std::collections::{BTreeMap, BTreeSet};

use ferrosift_model::{OperationId, OperationSpec};

mod support;

fn catalog() -> Vec<OperationSpec> {
    support::registry().catalog().cloned().collect()
}

/// A declared inverse must name an operation that exists.
///
/// Nothing checked this. `OperationSpec::validate` sees one specification at a
/// time and cannot resolve an id; the registry could, and did not. A typo in an
/// inverse would have produced a catalog that answers "the thing that undoes
/// this is `encoding.base46.encode@1`" and no test would have disagreed.
#[test]
fn every_declared_inverse_is_registered() {
    let catalog = catalog();
    let ids: BTreeSet<&str> = catalog.iter().map(|spec| spec.id.as_str()).collect();

    let mut declared = 0usize;
    for spec in &catalog {
        let Some(inverse) = &spec.inverse else {
            continue;
        };
        declared += 1;
        assert!(
            ids.contains(inverse.as_str()),
            "`{}` names `{}` as its inverse, and no such operation is registered",
            spec.id.as_str(),
            inverse.as_str()
        );
    }
    assert!(
        declared > 50,
        "the catalog is full of encoder/decoder couples; only {declared} declare an inverse, \
         which suggests this gate is reading the wrong field"
    );
}

/// If A undoes B then B undoes A.
///
/// A one-directional inverse is not a smaller claim than a symmetric one, it is
/// an inconsistent one: the catalog says the pair exists when read from one end
/// and does not when read from the other. Seven operations undo themselves and
/// satisfy this by naming their own id, which is the same rule and not an
/// exception to it.
#[test]
fn the_inverse_relation_is_symmetric() {
    let catalog = catalog();
    let by_id: BTreeMap<&str, &OperationSpec> = catalog
        .iter()
        .map(|spec| (spec.id.as_str(), spec))
        .collect();

    for spec in &catalog {
        let Some(inverse) = &spec.inverse else {
            continue;
        };
        let Some(partner) = by_id.get(inverse.as_str()) else {
            continue; // Reported by the gate above, with a better message.
        };
        assert_eq!(
            partner.inverse.as_ref().map(OperationId::as_str),
            Some(spec.id.as_str()),
            "`{}` says `{}` undoes it; `{}` does not say the same in return",
            spec.id.as_str(),
            inverse.as_str(),
            inverse.as_str()
        );
    }
}

/// An inverse lives in its own cluster.
///
/// This is what makes the namespace mean something. `encoding.base64.encode@1`
/// is undone by `encoding.base64.decode@1` and not by something in `hash`, and
/// an id that crossed the boundary would be either a naming mistake or a claim
/// worth arguing for in prose rather than asserting in a field.
#[test]
fn an_inverse_stays_in_its_own_cluster() {
    for spec in catalog() {
        let Some(inverse) = &spec.inverse else {
            continue;
        };
        assert_eq!(
            inverse.cluster(),
            spec.id.cluster(),
            "`{}` is in cluster `{}` and names an inverse in `{}`",
            spec.id.as_str(),
            spec.id.cluster(),
            inverse.cluster()
        );
    }
}

/// Every operation has siblings to be grouped with, even if it has none yet.
///
/// An id of one segment would cluster with itself and make the grouping
/// meaningless for that operation. The grammar allows it; the catalog does not
/// contain one, and this is what keeps it that way.
#[test]
fn every_operation_is_named_inside_a_namespace() {
    for spec in catalog() {
        let id = spec.id.as_str();
        assert_ne!(
            spec.id.cluster(),
            id,
            "`{id}` has no namespace to share with a sibling"
        );
        assert!(
            !spec.id.cluster().is_empty(),
            "`{id}` produced an empty cluster"
        );
    }
}

/// A cluster is one purpose, so its members agree about their category.
///
/// Three do not, and each is a merge the registry already documents: key
/// derivation sits with the ciphers it feeds, one text operation is a cipher,
/// and one data operation is text. They are listed rather than excluded by a
/// rule, so a fourth has to be argued for here before it can appear.
#[test]
fn a_cluster_spans_one_category_unless_it_is_a_named_merge() {
    const MERGED: &[(&str, &[&str])] = &[
        ("crypto", &["Ciphers", "KDF"]),
        ("data", &["Data", "Text"]),
        ("text", &["Ciphers", "Text"]),
    ];

    let mut categories: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let catalog = catalog();
    for spec in &catalog {
        categories
            .entry(spec.id.cluster())
            .or_default()
            .insert(spec.category.as_str());
    }

    let merged: BTreeMap<&str, BTreeSet<&str>> = MERGED
        .iter()
        .map(|(cluster, names)| (*cluster, names.iter().copied().collect()))
        .collect();

    for (cluster, found) in &categories {
        if found.len() == 1 {
            assert!(
                !merged.contains_key(cluster),
                "`{cluster}` is listed as spanning categories and now holds only one; \
                 drop the stale entry"
            );
            continue;
        }
        let expected = merged.get(cluster).unwrap_or_else(|| {
            panic!(
                "cluster `{cluster}` spans {found:?}; either the operations are misnamed \
                 or the merge belongs in this test's list with a reason"
            )
        });
        assert_eq!(
            found, expected,
            "`{cluster}` no longer spans the categories this test records"
        );
    }
}

/// The shape of the catalog, so a change to it is a visible one.
///
/// Not a target and not a limit — a record. Sixty couples and twenty
/// operations standing alone is a fact about what has been ported, and a
/// commit that moves it should say why in its message rather than move it
/// quietly.
#[test]
fn the_catalog_groups_into_clusters() {
    let catalog = catalog();
    let mut sizes: BTreeMap<&str, usize> = BTreeMap::new();
    for spec in &catalog {
        *sizes.entry(spec.id.cluster()).or_default() += 1;
    }

    let singletons = sizes.values().filter(|count| **count == 1).count();
    assert!(
        sizes.len() > 80 && sizes.len() < 130,
        "{} clusters over {} operations is outside the shape this catalog has had; \
         a large move is fine and should be explained",
        sizes.len(),
        catalog.len()
    );
    assert!(
        singletons * 4 < catalog.len(),
        "{singletons} of {} operations stand alone in their namespace, which is more \
         than this catalog has ever had; either a family was renamed or the ids drifted",
        catalog.len()
    );
}
