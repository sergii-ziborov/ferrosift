//! Exact alias, field, flag, and argument mapping tests.

mod support;

use ferrosift_compat::cyberchef::{MAX_ARGUMENT_DEPTH, import_recipe};
use ferrosift_model::{ArgumentKind, ArgumentValue, CompatibilityProfile};

use support::{argument, registry_with};

#[test]
fn supported_step_maps_exact_alias_arguments_and_flags() {
    let registry = registry_with(
        "test.transform@1",
        &["Test Transform"],
        vec![
            argument("enabled", ArgumentKind::Boolean, true, None),
            argument("count", ArgumentKind::Integer, true, None),
            argument("label", ArgumentKind::Text, false, None),
        ],
    );
    let source =
        br#"[{"op":"Test Transform","args":[true,7,"hello"],"disabled":true,"breakpoint":true}]"#;

    let report = import_recipe(source, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("bounded recipe parses");
    let recipe = report.recipe.expect("every source semantic is supported");
    let step = &recipe.steps[0];

    assert_eq!(step.id.as_str(), "cc-0000");
    assert_eq!(step.operation.as_str(), "test.transform@1");
    assert_eq!(step.arguments["enabled"], ArgumentValue::Boolean(true));
    assert_eq!(step.arguments["count"], ArgumentValue::Integer(7));
    assert_eq!(step.arguments["label"], ArgumentValue::Text("hello".into()));
    assert!(step.disabled);
    assert!(step.breakpoint);
    assert!(report.findings.is_empty());
}

#[test]
fn operation_alias_lookup_is_case_sensitive_and_never_guessed() {
    let registry = registry_with("test.transform@1", &["Test Transform"], vec![]);
    let report = import_recipe(
        br#"[{"op":"test transform","args":[]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("unknown operations remain preservable");

    assert!(report.recipe.is_none());
    assert_eq!(report.findings.len(), 1);
    assert_eq!(
        report.findings[0].code,
        "compat.cyberchef.unknown_operation"
    );
    assert_eq!(report.findings[0].source_step, 0);
    assert_eq!(
        report.findings[0].original_operation.as_deref(),
        Some("test transform")
    );
}

#[test]
fn invalid_step_fields_are_ordered_findings_and_fail_closed() {
    let registry = registry_with("test.empty@1", &["Empty"], vec![]);
    let source = br#"[null,{"args":[]},{"op":1,"args":[]},{"op":"Empty"},{"op":"Empty","args":{}},{"op":"Empty","args":[],"disabled":"yes","breakpoint":1,"zFuture":true}]"#;

    let report = import_recipe(source, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("step-level divergences are reportable");
    let codes: Vec<_> = report.findings.iter().map(|finding| finding.code).collect();
    let indices: Vec<_> = report
        .findings
        .iter()
        .map(|finding| finding.source_step)
        .collect();

    assert!(report.recipe.is_none());
    assert_eq!(
        codes,
        vec![
            "compat.cyberchef.step_not_object",
            "compat.cyberchef.missing_op",
            "compat.cyberchef.invalid_op",
            "compat.cyberchef.missing_args",
            "compat.cyberchef.invalid_args",
            "compat.cyberchef.invalid_disabled",
            "compat.cyberchef.invalid_breakpoint",
            "compat.cyberchef.unknown_field",
        ]
    );
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 5, 5]);
}

#[test]
fn argument_count_and_kind_divergences_are_explicit() {
    let registry = registry_with(
        "test.arguments@1",
        &["Arguments"],
        vec![
            argument("required", ArgumentKind::Boolean, true, None),
            argument(
                "defaulted",
                ArgumentKind::Integer,
                false,
                Some(ArgumentValue::Integer(9)),
            ),
        ],
    );
    let source = br#"[{"op":"Arguments","args":[]},{"op":"Arguments","args":["yes"]},{"op":"Arguments","args":[true,9,"extra"]}]"#;

    let report = import_recipe(source, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("argument divergences are reportable");
    let codes: Vec<_> = report.findings.iter().map(|finding| finding.code).collect();

    assert!(report.recipe.is_none());
    assert_eq!(
        codes,
        vec![
            "compat.cyberchef.missing_argument",
            "compat.cyberchef.argument_type",
            "compat.cyberchef.extra_argument",
        ]
    );
}

#[test]
fn bytes_lists_and_maps_convert_without_string_coercion() {
    let registry = registry_with(
        "test.structured@1",
        &["Structured"],
        vec![
            argument("bytes", ArgumentKind::Bytes, true, None),
            argument("list", ArgumentKind::List, true, None),
            argument("map", ArgumentKind::Map, true, None),
        ],
    );
    let source = br#"[{"op":"Structured","args":[[0,255],[true,7,"x"],{"nested":[1,false]}]}]"#;

    let report = import_recipe(source, CompatibilityProfile::CyberChefV11_3, &registry)
        .expect("structured JSON parses");
    let recipe = report.recipe.expect("all structures are representable");
    let arguments = &recipe.steps[0].arguments;

    assert_eq!(arguments["bytes"], ArgumentValue::Bytes(vec![0, 255]));
    assert!(matches!(arguments["list"], ArgumentValue::List(_)));
    assert!(matches!(arguments["map"], ArgumentValue::Map(_)));
}

#[test]
fn strings_are_not_guessed_to_be_bytes() {
    let registry = registry_with(
        "test.bytes@1",
        &["Bytes"],
        vec![argument("bytes", ArgumentKind::Bytes, true, None)],
    );
    let report = import_recipe(
        br#"[{"op":"Bytes","args":["abc"]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("type divergence is reportable");

    assert!(report.recipe.is_none());
    assert_eq!(report.findings[0].code, "compat.cyberchef.argument_type");
}

#[test]
fn javascript_unsafe_integers_never_become_executable_arguments() {
    let registry = registry_with(
        "test.integer@1",
        &["Integer"],
        vec![argument("value", ArgumentKind::Integer, true, None)],
    );
    let safe = import_recipe(
        br#"[{"op":"Integer","args":[9007199254740991]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("safe integer parses");
    let unsafe_value = import_recipe(
        br#"[{"op":"Integer","args":[9007199254740992]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("unsafe integer remains reportable");
    let safe_negative = import_recipe(
        br#"[{"op":"Integer","args":[-9007199254740991]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("negative safe integer parses");
    let unsafe_negative = import_recipe(
        br#"[{"op":"Integer","args":[-9007199254740992]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("negative unsafe integer remains reportable");
    let structured_registry = registry_with(
        "test.list@1",
        &["List"],
        vec![argument("value", ArgumentKind::List, true, None)],
    );
    let nested_unsafe = import_recipe(
        br#"[{"op":"List","args":[[{"number":9007199254740992}]]}]"#,
        CompatibilityProfile::CyberChefV11_3,
        &structured_registry,
    )
    .expect("nested unsafe integer remains reportable");

    assert!(safe.recipe.is_some());
    assert!(safe_negative.recipe.is_some());
    assert!(unsafe_value.recipe.is_none());
    assert!(unsafe_negative.recipe.is_none());
    assert!(nested_unsafe.recipe.is_none());
    assert_eq!(
        unsafe_value.findings[0].code,
        "compat.cyberchef.argument_number_range"
    );
    assert_eq!(
        unsafe_negative.findings[0].code,
        "compat.cyberchef.argument_number_range"
    );
    assert_eq!(
        nested_unsafe.findings[0].code,
        "compat.cyberchef.argument_number_range"
    );
}

#[test]
fn excessive_argument_depth_is_a_stable_finding() {
    let registry = registry_with(
        "test.list@1",
        &["List"],
        vec![argument("value", ArgumentKind::List, true, None)],
    );
    let nested = format!(
        "{}true{}",
        "[".repeat(MAX_ARGUMENT_DEPTH + 1),
        "]".repeat(MAX_ARGUMENT_DEPTH + 1)
    );
    let source = format!(r#"[{{"op":"List","args":[{nested}]}}]"#);

    let report = import_recipe(
        source.as_bytes(),
        CompatibilityProfile::CyberChefV11_3,
        &registry,
    )
    .expect("depth beyond executable limit remains preservable");

    assert!(report.recipe.is_none());
    assert_eq!(report.findings[0].code, "compat.cyberchef.argument_depth");
}
