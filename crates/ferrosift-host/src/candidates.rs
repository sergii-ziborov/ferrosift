//! Bounded candidate-recipe evaluation over one retained artifact.

use ferrosift_core::{ExecutionStatus, ValueSummary};
use ferrosift_model::{TextEncoding, Value, ValueKind};
use serde::{Deserialize, Serialize};

use crate::{
    artifact::ArtifactMeta,
    error::{HostError, HostResult},
    report::ReportStatus,
    service::RecipeFormat,
};

/// Maximum candidates accepted in one request.
pub const MAX_CANDIDATES: usize = 8;

/// One explicit recipe hypothesis supplied by the caller.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CandidateRecipe {
    /// Caller-chosen label for this hypothesis.
    pub id: String,
    /// Recipe dialect.
    pub format: String,
    /// Recipe JSON text.
    pub recipe_json: String,
    /// Optional checks applied to a successful output.
    #[serde(default)]
    pub checks: Vec<CandidateCheck>,
}

/// Deterministic post-conditions. These are observations, not calibrated probabilities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidateCheck {
    /// Output representation must match.
    ValueKind {
        /// Expected kind label such as `bytes` or `text`.
        expected: String,
    },
    /// Exact logical payload size.
    SizeEq {
        /// Expected size in bytes.
        bytes: u64,
    },
    /// Minimum logical payload size.
    SizeMin {
        /// Inclusive lower bound.
        bytes: u64,
    },
    /// Maximum logical payload size.
    SizeMax {
        /// Inclusive upper bound.
        bytes: u64,
    },
    /// Leading bytes of a bytes/UTF-8 text value, as hex.
    PrefixHex {
        /// Even-length hex string.
        hex: String,
    },
    /// Value must be UTF-8 text (or bytes that decode as UTF-8).
    Utf8Text,
}

/// Request to evaluate several recipes against one artifact.
#[derive(Clone, Debug)]
pub struct CandidatesRequest<'a> {
    /// Shared input artifact handle.
    pub input_artifact_id: &'a str,
    /// Explicit candidates, at most [`MAX_CANDIDATES`].
    pub candidates: &'a [CandidateRecipe],
}

/// Result table for one candidate batch.
#[derive(Clone, Debug, Serialize)]
pub struct CandidatesReport {
    /// Stable schema identifier.
    pub schema: &'static str,
    /// Shared input handle.
    pub input_artifact_id: String,
    /// Per-candidate outcomes in request order.
    pub results: Vec<CandidateResult>,
    /// Reminder that check counts are not calibrated probabilities.
    pub warnings: Vec<&'static str>,
}

/// Outcome for one candidate.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateResult {
    /// Caller-chosen label.
    pub id: String,
    /// Whether execution completed or paused.
    pub status: ReportStatus,
    /// Machine code when execution or checks failed.
    pub error_code: Option<String>,
    /// Human detail when failed.
    pub detail: Option<String>,
    /// Output artifact handle when a value was retained.
    pub artifact_id: Option<String>,
    /// Output representation when available.
    pub value_kind: Option<ValueKind>,
    /// Logical output size when available.
    pub output_size_bytes: Option<u64>,
    /// Individual check outcomes.
    pub checks: Vec<CheckOutcome>,
    /// Number of checks that passed.
    pub checks_passed: usize,
    /// Total checks requested.
    pub checks_total: usize,
}

/// One check observation.
#[derive(Clone, Debug, Serialize)]
pub struct CheckOutcome {
    /// Check kind label.
    pub kind: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Short observation detail.
    pub detail: String,
}

/// Validates candidate-batch bounds before execution.
///
/// # Errors
///
/// Returns [`HostError`] when the batch is empty or too large.
pub fn validate_batch(candidates: &[CandidateRecipe]) -> HostResult<()> {
    if candidates.is_empty() {
        return Err(HostError::new(
            "host.candidates.empty",
            "at least one candidate is required",
        ));
    }
    if candidates.len() > MAX_CANDIDATES {
        return Err(HostError::new(
            "host.candidates.too_many",
            format!("limit={MAX_CANDIDATES}"),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            return Err(HostError::new(
                "host.candidates.id_empty",
                "candidate id must not be empty",
            ));
        }
        if !seen.insert(candidate.id.as_str()) {
            return Err(HostError::new(
                "host.candidates.id_duplicate",
                candidate.id.clone(),
            ));
        }
    }
    Ok(())
}

/// Parses a recipe format label from a candidate document.
///
/// # Errors
///
/// Returns [`HostError`] for unknown labels.
pub fn parse_candidate_format(label: &str) -> HostResult<RecipeFormat> {
    match label {
        "ferrosift" => Ok(RecipeFormat::FerroSift),
        "cyberchef-v11.3" => Ok(RecipeFormat::CyberChefV11_3),
        "cyberchef-v11.4" => Ok(RecipeFormat::CyberChefV11_4),
        other => Err(HostError::new(
            "host.candidates.format_unknown",
            other.to_owned(),
        )),
    }
}

/// Evaluates configured checks against a retained value.
#[must_use]
pub fn evaluate_checks(value: &Value, checks: &[CandidateCheck]) -> Vec<CheckOutcome> {
    checks
        .iter()
        .map(|check| match check {
            CandidateCheck::ValueKind { expected } => {
                let actual = kind_label(value.kind());
                CheckOutcome {
                    kind: "value_kind".into(),
                    passed: actual == expected,
                    detail: format!("expected={expected} actual={actual}"),
                }
            }
            CandidateCheck::SizeEq { bytes } => {
                let size = ValueSummary::from_value(value).size_bytes;
                CheckOutcome {
                    kind: "size_eq".into(),
                    passed: size == *bytes,
                    detail: format!("expected={bytes} actual={size}"),
                }
            }
            CandidateCheck::SizeMin { bytes } => {
                let size = ValueSummary::from_value(value).size_bytes;
                CheckOutcome {
                    kind: "size_min".into(),
                    passed: size >= *bytes,
                    detail: format!("min={bytes} actual={size}"),
                }
            }
            CandidateCheck::SizeMax { bytes } => {
                let size = ValueSummary::from_value(value).size_bytes;
                CheckOutcome {
                    kind: "size_max".into(),
                    passed: size <= *bytes,
                    detail: format!("max={bytes} actual={size}"),
                }
            }
            CandidateCheck::PrefixHex { hex } => match decode_hex(hex) {
                Ok(prefix) => match value_bytes(value) {
                    Some(bytes) if bytes.starts_with(&prefix) => CheckOutcome {
                        kind: "prefix_hex".into(),
                        passed: true,
                        detail: format!("matched {} bytes", prefix.len()),
                    },
                    Some(_) => CheckOutcome {
                        kind: "prefix_hex".into(),
                        passed: false,
                        detail: "prefix mismatch".into(),
                    },
                    None => CheckOutcome {
                        kind: "prefix_hex".into(),
                        passed: false,
                        detail: "value is not bytes or UTF-8 text".into(),
                    },
                },
                Err(detail) => CheckOutcome {
                    kind: "prefix_hex".into(),
                    passed: false,
                    detail,
                },
            },
            CandidateCheck::Utf8Text => match value {
                Value::Text(text) if text.encoding == TextEncoding::Utf8 => CheckOutcome {
                    kind: "utf8_text".into(),
                    passed: true,
                    detail: "utf8 text".into(),
                },
                Value::Bytes(bytes) => match std::str::from_utf8(bytes) {
                    Ok(_) => CheckOutcome {
                        kind: "utf8_text".into(),
                        passed: true,
                        detail: "bytes are utf8".into(),
                    },
                    Err(_) => CheckOutcome {
                        kind: "utf8_text".into(),
                        passed: false,
                        detail: "bytes are not utf8".into(),
                    },
                },
                _ => CheckOutcome {
                    kind: "utf8_text".into(),
                    passed: false,
                    detail: format!("kind={}", kind_label(value.kind())),
                },
            },
        })
        .collect()
}

/// Builds a failed candidate row without creating an output artifact.
#[must_use]
pub fn failed_result(id: impl Into<String>, code: &str, detail: impl Into<String>) -> CandidateResult {
    CandidateResult {
        id: id.into(),
        status: ReportStatus::Completed,
        error_code: Some(code.to_owned()),
        detail: Some(detail.into()),
        artifact_id: None,
        value_kind: None,
        output_size_bytes: None,
        checks: Vec::new(),
        checks_passed: 0,
        checks_total: 0,
    }
}

/// Builds a successful or paused candidate row after retention.
#[must_use]
pub fn observed_result(
    id: impl Into<String>,
    status: ExecutionStatus,
    meta: &ArtifactMeta,
    checks: Vec<CheckOutcome>,
) -> CandidateResult {
    let checks_passed = checks.iter().filter(|check| check.passed).count();
    let checks_total = checks.len();
    let all_passed = checks_total == 0 || checks_passed == checks_total;
    CandidateResult {
        id: id.into(),
        status: ReportStatus::from(status),
        error_code: (!all_passed).then(|| String::from("host.candidates.checks_failed")),
        detail: (!all_passed).then(|| {
            format!("{checks_passed}/{checks_total} checks passed")
        }),
        artifact_id: Some(meta.id.as_str().to_owned()),
        value_kind: Some(meta.kind),
        output_size_bytes: Some(meta.size_bytes),
        checks,
        checks_passed,
        checks_total,
    }
}

fn value_bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::Bytes(bytes) => Some(bytes.clone()),
        Value::Text(text) if text.encoding == TextEncoding::Utf8 => {
            Some(text.text.as_bytes().to_vec())
        }
        _ => None,
    }
}

fn kind_label(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Empty => "empty",
        ValueKind::Bytes => "bytes",
        ValueKind::Text => "text",
        ValueKind::Boolean => "boolean",
        ValueKind::Integer => "integer",
        ValueKind::Number => "number",
        ValueKind::Decimal => "decimal",
        ValueKind::Markup => "markup",
        ValueKind::Structured => "structured",
        ValueKind::Files => "files",
    }
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err("prefix hex must have an even length".into());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&cleaned[index..index + 2], 16)
                .map_err(|_| String::from("prefix hex contains a non-hex digit"))
        })
        .collect()
}
