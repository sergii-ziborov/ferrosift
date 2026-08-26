use ferrosift_compat::cyberchef::import_recipe;
use ferrosift_core::{ExecutionStatus, Executor, NeverCancelled, TraceEventKind};
use ferrosift_model::{CapabilitySet, TextEncoding, Value, ValueKind};

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
        Value::Text(text) if text.encoding == TextEncoding::Utf8 => {
            encode_text_like_reference(&text.text)
        }
        // An operation whose reference returns a JavaScript number. The
        // reference's own harness renders it with `String(value)`, so the
        // comparison is against its decimal digits — which is also what a
        // caller sees when they print it. Modelling it as an integer rather
        // than as text is what stops the caller parsing a number back out of a
        // string to use it.
        Value::Integer(number) => number.to_string().into_bytes(),
        // A markup value is compared as the markup itself, because that is
        // what the reference's dish holds and what the harness now pins. The
        // stripped form is what a *later* step would receive, and pinning that
        // here would have let an operation emitting no tags at all pass.
        Value::Markup(markup) => encode_text_like_reference(&markup),
        // A number is compared as the digits the reference prints. Rendering
        // happens here rather than in the operation so that a caller receives
        // the number itself and does not have to parse one back out of a
        // string to use it.
        Value::Number(number) => match Value::Number(number).reinterpret(ValueKind::Text) {
            Some(Value::Text(text)) => encode_text_like_reference(&text.text),
            _ => Vec::new(),
        },
        // Rendered through the same projection a later step would see, which
        // is `JSON.stringify(value, null, 4)` -- so the four spaces are part
        // of what is compared.
        Value::Structured(value) => match Value::Structured(value).reinterpret(ValueKind::Text) {
            Some(Value::Text(text)) => encode_text_like_reference(&text.text),
            _ => Vec::new(),
        },
        other => panic!(
            "reference normalization does not support {:?}",
            other.kind()
        ),
    }
}

/// The reference's `Utils.strToByteArray`: a string whose UTF-16 code units
/// all fit in one byte is observed as Latin-1, anything else as UTF-8.
fn encode_text_like_reference(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match u8::try_from(unit) {
            Ok(byte) => bytes.push(byte),
            Err(_) => return text.as_bytes().to_vec(),
        }
    }
    bytes
}
