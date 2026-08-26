//! Execution trace summaries must remain bounded and representation-aware.

use std::collections::BTreeMap;

use ferrosift_core::{
    ExecutionStatus, ExecutionTrace, StepLocation, TraceEvent, TraceEventKind, ValueSummary,
};
use ferrosift_model::{
    OperationId, StepId, StructuredValue, TextEncoding, TextValue, Value, ValueKind, VirtualFile,
};

#[test]
fn summaries_measure_logical_payloads_without_retaining_values() {
    let values = [
        (Value::Bytes(vec![0, 1, 2]), ValueKind::Bytes, 3),
        (
            Value::Text(TextValue {
                text: "שלום".into(),
                encoding: TextEncoding::Utf8,
            }),
            ValueKind::Text,
            8,
        ),
        (
            Value::Structured(StructuredValue::Object(Vec::from([
                ("a".into(), StructuredValue::Boolean(true)),
                (
                    "items".into(),
                    StructuredValue::List(vec![
                        StructuredValue::Integer(7),
                        StructuredValue::Bytes(vec![1, 2]),
                    ]),
                ),
            ]))),
            ValueKind::Structured,
            25,
        ),
        (
            Value::Files(vec![VirtualFile {
                name: "a.bin".into(),
                media_type: Some("application/octet-stream".into()),
                contents: vec![1, 2, 3],
            }]),
            ValueKind::Files,
            32,
        ),
    ];

    for (value, kind, size_bytes) in values {
        assert_eq!(
            ValueSummary::from_value(&value),
            ValueSummary { kind, size_bytes }
        );
    }
}

#[test]
fn trace_events_carry_only_locations_and_summaries() {
    let location = StepLocation {
        index: 0,
        step_id: StepId::new("decode").expect("valid step ID"),
        operation: OperationId::new("encoding.decode@1").expect("valid operation ID"),
    };
    let input = ValueSummary {
        kind: ValueKind::Bytes,
        size_bytes: 4,
    };
    let trace = ExecutionTrace {
        events: vec![TraceEvent {
            location: location.clone(),
            kind: TraceEventKind::StepStarted { input },
        }],
    };

    assert_eq!(trace.events[0].location, location);
    assert_eq!(trace.events[0].kind, TraceEventKind::StepStarted { input });
    assert_eq!(
        ExecutionStatus::Paused { step_index: 0 },
        ExecutionStatus::Paused { step_index: 0 }
    );
    assert_ne!(
        ExecutionStatus::Paused { step_index: 0 },
        ExecutionStatus::Completed
    );
}
