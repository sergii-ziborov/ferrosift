use ferrosift_compat::cyberchef::import_recipe;
use ferrosift_core::{ExecutionStatus, Executor, NeverCancelled, TraceEventKind};
use ferrosift_model::{CapabilitySet, TextEncoding, Value};

use super::fixture::{Case, UnsupportedCase, decode_hex};

pub fn assert_supported_case(case: &Case) {
    assert_eq!(
        case.outputs_hex.len(),
        case.recipe.len(),
        "{} must observe every recipe prefix",
        case.name
    );
    assert_eq!(
        case.stopped_after,
        case.recipe.len(),
        "{} reference must complete the recipe",
        case.name
    );

    for prefix_length in 1..=case.recipe.len() {
        assert_prefix(case, prefix_length);
    }
}

pub fn assert_unsupported_case(case: &UnsupportedCase) {
    let registry = ferrosift_operations::default_registry()
        .expect("built-in operation registry must validate");
    let source = serde_json::to_vec(&case.recipe).expect("recipe must serialize");
    let report = import_recipe(&source, &registry).expect("recipe must parse");

    assert!(report.recipe.is_none(), "{} must not execute", case.name);
    assert_eq!(report.findings.len(), 1, "{} finding count", case.name);
    let finding = &report.findings[0];
    assert_eq!(
        finding.code, case.finding.code,
        "{} finding code",
        case.name
    );
    assert_eq!(
        finding.source_step, case.finding.source_step,
        "{} finding location",
        case.name
    );
    assert_eq!(
        finding.original_operation.as_deref(),
        Some(case.finding.original_operation.as_str()),
        "{} original operation",
        case.name
    );
}

fn assert_prefix(case: &Case, prefix_length: usize) {
    let registry = ferrosift_operations::default_registry()
        .expect("built-in operation registry must validate");
    let source =
        serde_json::to_vec(&case.recipe[..prefix_length]).expect("recipe prefix must serialize");
    let report = import_recipe(&source, &registry).expect("recipe prefix must parse");
    assert!(
        report.findings.is_empty(),
        "{} prefix {prefix_length}: {:?}",
        case.name,
        report.findings
    );
    let recipe = report.recipe.expect("supported prefix must map");
    let result = Executor::new(&registry)
        .execute(
            &recipe,
            case.input.to_value(),
            crate::support::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("supported prefix must execute");

    assert_eq!(
        result.status,
        ExecutionStatus::Completed,
        "{} prefix {prefix_length} status",
        case.name
    );
    assert_eq!(
        completed_steps(&result.trace.events),
        prefix_length,
        "{} prefix {prefix_length} stopping position",
        case.name
    );
    assert_eq!(
        normalize(result.value),
        decode_hex(&case.outputs_hex[prefix_length - 1]),
        "{} prefix {prefix_length} output",
        case.name
    );
}

fn completed_steps(events: &[ferrosift_core::TraceEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event.kind, TraceEventKind::StepCompleted { .. }))
        .count()
}

fn normalize(value: Value) -> Vec<u8> {
    match value {
        Value::Bytes(bytes) => bytes,
        Value::Text(text) if text.encoding == TextEncoding::Utf8 => text.text.into_bytes(),
        other => panic!(
            "reference normalization does not support {:?}",
            other.kind()
        ),
    }
}
