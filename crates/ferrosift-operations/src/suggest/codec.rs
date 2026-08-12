//! Deterministic recipe suggestions (Magic-as-advisor).
//!
//! This never mutates the input into a "best" decode. It ranks portable
//! operations already in the catalog and reports candidate recipes.

use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::Write as _;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentValue, Arguments, TextEncoding, TextValue, Value};

use crate::failure::failed;
use crate::{
    Bzip2Decompress, FangUrl, FromBase32, FromBase64, FromHex, FromHtmlEntity, Gunzip, RawInflate,
    Rot13, UrlDecode, ZlibInflate,
};

const MAX_DEPTH: i128 = 3;
const MAX_RESULTS_CAP: i128 = 32;
const PREVIEW_CHARS: usize = 64;

#[derive(Clone, Copy)]
struct Step {
    id: &'static str,
    alias: &'static str,
    args_summary: &'static str,
    recipe_fragment: &'static str,
}

struct Hit {
    score: u16,
    steps: Vec<Step>,
    reason: String,
    preview: String,
}

#[derive(Clone, Copy)]
struct Options {
    depth: usize,
    intensive: bool,
}

pub(super) fn suggest(
    input: Value,
    depth: i128,
    max_results: i128,
    intensive: bool,
    crib: &str,
    context: &mut OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(failed("analysis.suggest.invalid_depth"));
    }
    if max_results <= 0 || max_results > MAX_RESULTS_CAP {
        return Err(failed("analysis.suggest.invalid_max_results"));
    }
    let max_results =
        usize::try_from(max_results).map_err(|_| failed("analysis.suggest.invalid_max_results"))?;
    let options = Options {
        depth: usize::try_from(depth).unwrap_or(1),
        intensive,
    };

    let (bytes, text) = normalize_input(input)?;
    let mut hits = Vec::new();
    explore(&bytes, text.as_deref(), options, &mut hits, context)?;

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
        a.steps.iter().map(|s| s.id).eq(b.steps.iter().map(|s| s.id)) && a.preview == b.preview
    });
    hits.truncate(max_results);

    let report = render_report(&bytes, &hits);
    ensure_budget(report.len(), context)?;
    context.ensure_active()?;
    Ok(report)
}

fn normalize_input(input: Value) -> Result<(Vec<u8>, Option<String>), OperationError> {
    match input {
        Value::Bytes(bytes) => {
            let text = core::str::from_utf8(&bytes).ok().map(String::from);
            Ok((bytes, text))
        }
        Value::Text(text) => Ok((text.text.as_bytes().to_vec(), Some(text.text))),
        _ => Err(OperationError::InvalidArguments),
    }
}

fn explore(
    bytes: &[u8],
    text: Option<&str>,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    context.ensure_active()?;
    if bytes.is_empty() {
        return Ok(());
    }
    probe_magic_bytes(bytes, options, hits, context)?;
    if let Some(text) = text {
        probe_text(text, options, hits, context)?;
    } else if let Ok(lossy) = core::str::from_utf8(bytes) {
        probe_text(lossy, options, hits, context)?;
    }
    if options.intensive && looks_mostly_alpha(bytes) {
        try_op(
            Value::Bytes(bytes.to_vec()),
            Step {
                id: "encoding.rot13@1",
                alias: "ROT13",
                args_summary: "amount=13",
                recipe_fragment: r#"{"op":"ROT13","args":[true,true,false,13]}"#,
            },
            "mostly alphabetic; ROT13 may surface plaintext",
            options,
            hits,
            context,
            |input, ctx| {
                Rot13::new().execute(
                    input,
                    &Arguments::from([
                        ("rotate_lower_case_chars".into(), ArgumentValue::Boolean(true)),
                        ("rotate_upper_case_chars".into(), ArgumentValue::Boolean(true)),
                        ("rotate_numbers".into(), ArgumentValue::Boolean(false)),
                        ("amount".into(), ArgumentValue::Integer(13)),
                    ]),
                    ctx,
                )
            },
        )?;
    }
    Ok(())
}

fn probe_magic_bytes(
    bytes: &[u8],
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        try_op(
            Value::Bytes(bytes.to_vec()),
            Step {
                id: "compression.gunzip@1",
                alias: "Gunzip",
                args_summary: "(none)",
                recipe_fragment: r#"{"op":"Gunzip","args":[]}"#,
            },
            "gzip magic 1f 8b",
            options,
            hits,
            context,
            |input, ctx| Gunzip::new().execute(input, &Arguments::new(), ctx),
        )?;
    }
    if bytes.starts_with(b"BZh") {
        try_op(
            Value::Bytes(bytes.to_vec()),
            Step {
                id: "compression.bzip2.decompress@1",
                alias: "Bzip2 Decompress",
                args_summary: "low_memory=false",
                recipe_fragment: r#"{"op":"Bzip2 Decompress","args":[false]}"#,
            },
            "bzip2 magic BZh",
            options,
            hits,
            context,
            |input, ctx| {
                Bzip2Decompress::new().execute(
                    input,
                    &Arguments::from([("low_memory".into(), ArgumentValue::Boolean(false))]),
                    ctx,
                )
            },
        )?;
    }
    if looks_like_zlib(bytes) {
        try_op(
            Value::Bytes(bytes.to_vec()),
            Step {
                id: "compression.zlib.inflate@1",
                alias: "Zlib Inflate",
                args_summary: "start_index=0",
                recipe_fragment: r#"{"op":"Zlib Inflate","args":[0,0,"Adaptive",false,false]}"#,
            },
            "zlib CMF/FLG header",
            options,
            hits,
            context,
            |input, ctx| ZlibInflate::new().execute(input, &zlib_args(), ctx),
        )?;
        try_op(
            Value::Bytes(bytes.to_vec()),
            Step {
                id: "compression.raw.inflate@1",
                alias: "Raw Inflate",
                args_summary: "start_index=0",
                recipe_fragment: r#"{"op":"Raw Inflate","args":[0,0,"Adaptive",false,false]}"#,
            },
            "possible raw deflate stream",
            options,
            hits,
            context,
            |input, ctx| RawInflate::new().execute(input, &zlib_args(), ctx),
        )?;
    }
    Ok(())
}

fn probe_text(
    text: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    probe_hex(text, trimmed, options, hits, context)?;
    probe_base64(text, trimmed, options, hits, context)?;
    probe_base32(text, trimmed, options, hits, context)?;
    probe_url(text, trimmed, options, hits, context)?;
    probe_html(text, trimmed, options, hits, context)?;
    probe_defang(text, trimmed, options, hits, context)?;
    Ok(())
}

fn probe_hex(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_like_hex(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "encoding.hex.decode@1",
            alias: "From Hex",
            args_summary: "delimiter=Auto",
            recipe_fragment: r#"{"op":"From Hex","args":["Auto"]}"#,
        },
        "high density of hex digits",
        options,
        hits,
        context,
        |input, ctx| {
            FromHex::new().execute(
                input,
                &Arguments::from([("delimiter".into(), ArgumentValue::Text("Auto".into()))]),
                ctx,
            )
        },
    )
}

fn probe_base64(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_like_base64(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "encoding.base64.decode@1",
            alias: "From Base64",
            args_summary: "alphabet=A-Za-z0-9+/=",
            recipe_fragment: r#"{"op":"From Base64","args":["A-Za-z0-9+/=",true,false]}"#,
        },
        "base64 alphabet and padding shape",
        options,
        hits,
        context,
        |input, ctx| {
            FromBase64::new().execute(
                input,
                &Arguments::from([
                    (
                        "alphabet".into(),
                        ArgumentValue::Text("A-Za-z0-9+/=".into()),
                    ),
                    ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
                    ("strict".into(), ArgumentValue::Boolean(false)),
                ]),
                ctx,
            )
        },
    )
}

fn probe_base32(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_like_base32(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "encoding.base32.decode@1",
            alias: "From Base32",
            args_summary: "alphabet=A-Z2-7=",
            recipe_fragment: r#"{"op":"From Base32","args":["A-Z2-7=",true]}"#,
        },
        "base32 alphabet density",
        options,
        hits,
        context,
        |input, ctx| {
            FromBase32::new().execute(
                input,
                &Arguments::from([
                    ("alphabet".into(), ArgumentValue::Text("A-Z2-7=".into())),
                    ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
                ]),
                ctx,
            )
        },
    )
}

fn probe_url(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_like_url_encoded(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "encoding.url.decode@1",
            alias: "URL Decode",
            args_summary: "treat_plus_as_space=true",
            recipe_fragment: r#"{"op":"URL Decode","args":[true]}"#,
        },
        "percent-encoded sequences present",
        options,
        hits,
        context,
        |input, ctx| {
            UrlDecode::new().execute(
                input,
                &Arguments::from([("treat_plus_as_space".into(), ArgumentValue::Boolean(true))]),
                ctx,
            )
        },
    )
}

fn probe_html(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_like_html_entities(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "encoding.html.decode@1",
            alias: "From HTML Entity",
            args_summary: "(none)",
            recipe_fragment: r#"{"op":"From HTML Entity","args":[]}"#,
        },
        "HTML entities present",
        options,
        hits,
        context,
        |input, ctx| FromHtmlEntity::new().execute(input, &Arguments::new(), ctx),
    )
}

fn probe_defang(
    text: &str,
    trimmed: &str,
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
    if !looks_defanged(trimmed) {
        return Ok(());
    }
    try_text(
        text,
        Step {
            id: "defang.fang_url@1",
            alias: "Fang URL",
            args_summary: "restore dots/hxxp/slashes",
            recipe_fragment: r#"{"op":"Fang URL","args":[true,true,true]}"#,
        },
        "defanged URL markers (hxxp / [.] / [://])",
        options,
        hits,
        context,
        |input, ctx| {
            FangUrl::new().execute(
                input,
                &Arguments::from([
                    ("restore_dots".into(), ArgumentValue::Boolean(true)),
                    ("restore_hxxp".into(), ArgumentValue::Boolean(true)),
                    ("restore_slashes".into(), ArgumentValue::Boolean(true)),
                ]),
                ctx,
            )
        },
    )
}

fn try_text<F>(
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

#[allow(clippy::too_many_arguments)]
fn try_op<F>(
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

fn render_report(bytes: &[u8], hits: &[Hit]) -> String {
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

fn looks_like_hex(text: &str) -> bool {
    let mut digits = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_hexdigit() {
            digits += 1;
        } else if ch.is_ascii_whitespace() || matches!(ch, ':' | ',' | '-' | 'x' | 'X' | '0') {
        } else {
            other += 1;
        }
    }
    digits >= 8 && digits.is_multiple_of(2) && other * 4 <= digits
}

fn looks_like_base64(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.len() < 8 {
        return false;
    }
    let body = compact.trim_end_matches('=');
    if !body
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    {
        return false;
    }
    compact.len().is_multiple_of(4)
}

fn looks_like_base32(text: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 8 {
        return false;
    }
    let body = compact.trim_end_matches('=');
    body.chars()
        .all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
        && body.len() * 5 >= 40
}

fn looks_like_url_encoded(text: &str) -> bool {
    text.contains('%')
        && text.as_bytes().windows(3).any(|w| {
            w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit()
        })
}

fn looks_like_html_entities(text: &str) -> bool {
    text.contains("&lt;")
        || text.contains("&gt;")
        || text.contains("&amp;")
        || text.contains("&#")
        || text.contains("&quot;")
}

fn looks_defanged(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("hxxp") || lower.contains("[.]") || lower.contains("[://]")
}

fn looks_like_zlib(bytes: &[u8]) -> bool {
    if bytes.len() < 2 {
        return false;
    }
    let cmf = bytes[0];
    let flg = bytes[1];
    cmf & 0x0f == 8 && (u16::from(cmf) * 256 + u16::from(flg)).is_multiple_of(31)
}

fn looks_mostly_alpha(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let alpha = bytes
        .iter()
        .filter(|b| b.is_ascii_alphabetic() || b.is_ascii_whitespace())
        .count();
    alpha * 100 / bytes.len() >= 80
}

fn zlib_args() -> Arguments {
    Arguments::from([
        ("start_index".into(), ArgumentValue::Integer(0)),
        (
            "initial_output_buffer_size".into(),
            ArgumentValue::Integer(0),
        ),
        (
            "buffer_expansion_type".into(),
            ArgumentValue::Text("Adaptive".into()),
        ),
        (
            "resize_buffer_after_decompression".into(),
            ArgumentValue::Boolean(false),
        ),
        ("verify_result".into(), ArgumentValue::Boolean(false)),
    ])
}

fn ensure_budget(len: usize, context: &OperationContext<'_>) -> Result<(), OperationError> {
    if u64::try_from(len).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
