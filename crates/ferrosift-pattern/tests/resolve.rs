//! Caller-supplied resolvers for `import` / `#include`.

use ferrosift_pattern::{
    EvalOptions, MapResolver, NodeValue, ResolveLimits, SourceOrigin, evaluate, parse, parse_with,
};

#[test]
fn single_source_parse_still_refuses_imports_without_a_resolver() {
    let error = parse("import std.io;\nu8 a @ 0;").expect_err("no ambient filesystem");
    assert_eq!(error.code(), "pattern.parse.unsupported_directive");
}

#[test]
fn import_and_include_merge_declarations_with_distinct_origins() {
    let mut resolver = MapResolver::new();
    resolver.insert("std.io", "using Byte = u8;\n");
    resolver.insert("helpers.pat", "struct Helper { Byte value; };\n");

    let pattern = parse_with(
        "main.hexpat",
        "import std.io;\n#include \"helpers.pat\"\nHelper h @ 0;",
        &resolver,
        ResolveLimits::default(),
    )
    .expect("resolves");

    assert_eq!(pattern.sources.len(), 3);
    assert!(matches!(pattern.sources[0].origin, SourceOrigin::Root));
    assert!(matches!(
        pattern.sources[1].origin,
        SourceOrigin::Import { .. }
    ));
    assert!(matches!(
        pattern.sources[2].origin,
        SourceOrigin::Include { .. }
    ));
    assert_eq!(pattern.sources[1].label, "std.io");
    assert_eq!(pattern.sources[2].label, "helpers.pat");

    let nodes = evaluate(&pattern, &[0x2a], &EvalOptions::default()).expect("eval");
    assert_eq!(nodes[0].child("value").unwrap().value, NodeValue::Unsigned(0x2a));
}

#[test]
fn positions_from_included_sources_carry_non_root_source_ids() {
    let mut resolver = MapResolver::new();
    resolver.insert("lib.pat", "u8 remote @ 0;\n");
    let pattern = parse_with(
        "root.hexpat",
        "#include <lib.pat>\n",
        &resolver,
        ResolveLimits::default(),
    )
    .expect("resolves");
    let placement = match &pattern.declarations[0] {
        ferrosift_pattern::Declaration::Placement(placement) => placement,
        other => panic!("expected placement, got {other:?}"),
    };
    assert_eq!(placement.position.source.index(), 1);
}

#[test]
fn cycles_and_missing_specifiers_fail_closed() {
    let mut resolver = MapResolver::new();
    resolver.insert("a.pat", "#include <b.pat>\n");
    resolver.insert("b.pat", "#include <a.pat>\n");
    let error = parse_with("root", "#include <a.pat>\n", &resolver, ResolveLimits::default())
        .expect_err("cycle");
    assert_eq!(error.code(), "pattern.resolve.cycle");

    let empty = MapResolver::new();
    let missing = parse_with(
        "root",
        "import missing.mod;\nu8 a @ 0;",
        &empty,
        ResolveLimits::default(),
    )
    .expect_err("missing");
    assert_eq!(missing.code(), "pattern.resolve.not_found");
}

#[test]
fn resolve_ceilings_are_enforced() {
    let mut resolver = MapResolver::new();
    resolver.insert("child.pat", "u8 a @ 0;\n");
    let limits = ResolveLimits {
        max_depth: 0,
        max_sources: 32,
        max_total_bytes: 1_048_576,
    };
    let error = parse_with("root", "#include <child.pat>\n", &resolver, limits)
        .expect_err("depth");
    assert_eq!(error.code(), "pattern.resolve.depth_exceeded");
}

#[test]
fn import_kind_is_recorded_separately_from_include() {
    let mut resolver = MapResolver::new();
    resolver.insert("a.b", "using X = u8;\n");
    resolver.insert("c.pat", "using Y = u16;\n");
    let pattern = parse_with(
        "root",
        "import a.b;\n#include <c.pat>\nX x @ 0;",
        &resolver,
        ResolveLimits::default(),
    )
    .expect("ok");
    assert!(matches!(
        pattern.sources[1].origin,
        SourceOrigin::Import {
            specifier: ref spec,
            ..
        } if spec == "a.b"
    ));
    assert!(matches!(
        pattern.sources[2].origin,
        SourceOrigin::Include {
            specifier: ref spec,
            ..
        } if spec == "c.pat"
    ));
}
