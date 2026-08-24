//! Scoring, previews, and the rendered advisory report.

use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::Write as _;

use ferrosift_core::{OperationContext, OperationError};
use ferrosift_model::{TextEncoding, TextValue, Value};

use super::detect::looks_like_zlib;
use super::model::{Hit, Options, PREVIEW_CHARS, Step};
use super::probes::explore;

/// Runs one candidate over text input and records it if it scores well.
pub(super) fn try_text<F>(
    text: &str,
    step: Step,
    reason: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
    run: F,
) -> Result<(), OperationError>
where
    F: FnOnce(Value, &mut OperationContext<'_>) -> Result<Value, OperationError>,
{
    try_op(
        Value::Text(TextValue {
            text: String::from(text),
            encoding: TextEncoding::Utf8,
        }),
        step,
        reason,
        options,
        hits,
        context,
        run,
    )
}

/// Runs one candidate; a failing operation is simply not a suggestion.
#[allow(clippy::too_many_arguments)]
pub(super) fn try_op<F>(
    input: Value,
    step: Step,
    reason: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
    run: F,
) -> Result<(), OperationError>
where
    F: FnOnce(Value, &mut OperationContext<'_>) -> Result<Value, OperationError>,
{
    match run(input, context) {
        Ok(output) => record_hit(step, reason, &output, options, hits, context),
        Err(_) => Ok(()),
    }
}

/// Applies the crib filter, then orders, de-duplicates, and truncates.
///
/// Ordering is total and deterministic: score first, then the leading
/// operation ID, then the preview, so equal-scoring candidates never depend
/// on discovery order.
pub(super) fn rank(hits: &mut Vec<Hit>, crib: &str, max_results: usize) {
    if !crib.is_empty() {
        let needle = crib.to_ascii_lowercase();
        hits.retain(|hit| {
            hit.preview.to_ascii_lowercase().contains(&needle)
                || hit.reason.to_ascii_lowercase().contains(&needle)
        });
    }
    hits.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.steps[0].id.cmp(right.steps[0].id))
            .then_with(|| left.preview.cmp(&right.preview))
    });
    hits.dedup_by(|a, b| {
        a.steps
            .iter()
            .map(|s| s.id)
            .eq(b.steps.iter().map(|s| s.id))
            && a.preview == b.preview
    });
    hits.truncate(max_results);
}

fn record_hit(
    step: Step,
    reason: &str,
    output: &Value,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    context.ensure_active()?;
    let (out_bytes, out_text) = value_views(output);
    let score = score_output(&out_bytes, out_text.as_deref());
    if score < 35 {
        return Ok(());
    }
    let preview = preview_of(&out_bytes, out_text.as_deref());
    hits.push(Hit {
        score,
        steps: vec![step],
        reason: String::from(reason),
        preview,
    });

    if options.depth > 1 && !out_bytes.is_empty() {
        let mut nested = Vec::new();
        let nested_options = Options {
            depth: options.depth - 1,
            intensive: options.intensive,
        };
        explore(
            &out_bytes,
            out_text.as_deref(),
            nested_options,
            &mut nested,
            context,
        )?;
        for child in nested {
            if child.score < 40 {
                continue;
            }
            let mut steps = vec![step];
            steps.extend(child.steps);
            hits.push(Hit {
                score: score.saturating_add(child.score / 2).min(99),
                steps,
                reason: format!("{reason}; then {}", child.reason),
                preview: child.preview,
            });
        }
    }
    Ok(())
}

fn value_views(value: &Value) -> (Vec<u8>, Option<String>) {
    match value {
        Value::Bytes(bytes) => {
            let text = core::str::from_utf8(bytes).ok().map(String::from);
            (bytes.clone(), text)
        }
        Value::Text(text) => (text.text.as_bytes().to_vec(), Some(text.text.clone())),
        _ => (Vec::new(), None),
    }
}

fn score_output(bytes: &[u8], text: Option<&str>) -> u16 {
    if bytes.is_empty() {
        return 0;
    }
    let printable = bytes
        .iter()
        .filter(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
        .count();
    let ratio = u16::try_from(printable.saturating_mul(100) / bytes.len()).unwrap_or(0);
    let mut score = ratio;
    if text.is_some() {
        score = score.saturating_add(15);
    }
    if bytes.starts_with(&[0x1f, 0x8b]) || bytes.starts_with(b"BZh") || looks_like_zlib(bytes) {
        score = score.max(70);
    }
    if ratio >= 85 {
        score = score.saturating_add(10);
    }
    score.min(99)
}

fn preview_of(bytes: &[u8], text: Option<&str>) -> String {
    if let Some(text) = text {
        let mut preview: String = text.chars().take(PREVIEW_CHARS).collect();
        if text.chars().count() > PREVIEW_CHARS {
            preview.push_str("...");
        }
        return escape_preview(&preview);
    }
    let take = bytes.len().min(24);
    let mut out = String::from("hex:");
    for byte in &bytes[..take] {
        out.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        out.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
    }
    if bytes.len() > take {
        out.push_str("...");
    }
    out
}

fn escape_preview(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push('.'),
            c => out.push(c),
        }
    }
    out
}

pub(super) fn render_report(bytes: &[u8], hits: &[Hit]) -> String {
    let printable = if bytes.is_empty() {
        0
    } else {
        bytes
            .iter()
            .filter(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
            .count()
            * 100
            / bytes.len()
    };
    let mut out = String::new();
    out.push_str("FerroSift Suggest recipe\n");
    let _ = writeln!(out, "input_bytes={} printable={printable}%", bytes.len());
    out.push_str(
        "note: CyberChef Magic remains unsupported; this advisor only ranks portable recipes\n",
    );
    if hits.is_empty() {
        out.push_str("(no suggestions)\n");
        return out;
    }
    out.push('\n');
    for (index, hit) in hits.iter().enumerate() {
        let rank = index + 1;
        let head = &hit.steps[0];
        let _ = writeln!(
            out,
            "{rank}. score={} {} ({})",
            hit.score, head.alias, head.id
        );
        if hit.steps.len() > 1 {
            let chain: Vec<&str> = hit.steps.iter().map(|s| s.alias).collect();
            let _ = writeln!(out, "   chain: {}", chain.join(" -> "));
        }
        let _ = writeln!(out, "   args: {}", head.args_summary);
        let _ = writeln!(out, "   reason: {}", hit.reason);
        let _ = writeln!(out, "   preview: {}", hit.preview);
        let recipe = hit
            .steps
            .iter()
            .map(|s| s.recipe_fragment)
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(out, "   recipe: [{recipe}]");
        if index + 1 < hits.len() {
            out.push('\n');
        }
    }
    out
}
