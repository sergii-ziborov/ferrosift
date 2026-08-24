//! Candidate discovery: magic-byte probes and text-shape probes.

use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentValue, Arguments, Value};

use super::detect::{
    looks_defanged, looks_like_base32, looks_like_base64, looks_like_hex, looks_like_html_entities,
    looks_like_url_encoded, looks_like_zlib, looks_mostly_alpha, zlib_args,
};
use super::model::{Hit, Options, Step};
use super::scoring::{try_op, try_text};
use crate::{
    Bzip2Decompress, FangUrl, FromBase32, FromBase64, FromHex, FromHtmlEntity, Gunzip, RawInflate,
    Rot13, UrlDecode, ZlibInflate,
};

pub(super) fn explore(
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
        probe_rot13(bytes, options, hits, context)?;
    }
    Ok(())
}

fn probe_rot13(
    bytes: &[u8],
    options: Options,
    hits: &mut Vec<Hit>,
    context: &mut OperationContext<'_>,
) -> Result<(), OperationError> {
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
                    (
                        "rotate_lower_case_chars".into(),
                        ArgumentValue::Boolean(true),
                    ),
                    (
                        "rotate_upper_case_chars".into(),
                        ArgumentValue::Boolean(true),
                    ),
                    ("rotate_numbers".into(), ArgumentValue::Boolean(false)),
                    ("amount".into(), ArgumentValue::Integer(13)),
                ]),
                ctx,
            )
        },
    )
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

/// Runs every text-shape probe in a fixed order.
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

/// Declares one text-shape probe.
///
/// Every probe has the same shape — guard on a detector, then offer one
/// operation with fixed arguments — so the shape is written once here rather
/// than repeated for each encoding.
macro_rules! text_probe {
    ($name:ident, $guard:path, $reason:literal, $operation:ty, $args:expr,
     id: $id:literal, alias: $alias:literal,
     summary: $summary:literal, fragment: $fragment:literal) => {
        fn $name(
            text: &str,
            trimmed: &str,
            options: Options,
            hits: &mut Vec<Hit>,
            context: &mut OperationContext<'_>,
        ) -> Result<(), OperationError> {
            if !$guard(trimmed) {
                return Ok(());
            }
            try_text(
                text,
                Step {
                    id: $id,
                    alias: $alias,
                    args_summary: $summary,
                    recipe_fragment: $fragment,
                },
                $reason,
                options,
                hits,
                context,
                |input, ctx| <$operation>::new().execute(input, &$args, ctx),
            )
        }
    };
}

text_probe!(
    probe_hex,
    looks_like_hex,
    "high density of hex digits",
    FromHex,
    Arguments::from([("delimiter".into(), ArgumentValue::Text("Auto".into()))]),
    id: "encoding.hex.decode@1",
    alias: "From Hex",
    summary: "delimiter=Auto",
    fragment: r#"{"op":"From Hex","args":["Auto"]}"#
);

text_probe!(
    probe_base64,
    looks_like_base64,
    "base64 alphabet and padding shape",
    FromBase64,
    Arguments::from([
        (
            "alphabet".into(),
            ArgumentValue::Text("A-Za-z0-9+/=".into()),
        ),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
        ("strict".into(), ArgumentValue::Boolean(false)),
    ]),
    id: "encoding.base64.decode@1",
    alias: "From Base64",
    summary: "alphabet=A-Za-z0-9+/=",
    fragment: r#"{"op":"From Base64","args":["A-Za-z0-9+/=",true,false]}"#
);

text_probe!(
    probe_base32,
    looks_like_base32,
    "base32 alphabet density",
    FromBase32,
    Arguments::from([
        ("alphabet".into(), ArgumentValue::Text("A-Z2-7=".into())),
        ("remove_non_alphabet".into(), ArgumentValue::Boolean(true)),
    ]),
    id: "encoding.base32.decode@1",
    alias: "From Base32",
    summary: "alphabet=A-Z2-7=",
    fragment: r#"{"op":"From Base32","args":["A-Z2-7=",true]}"#
);

text_probe!(
    probe_url,
    looks_like_url_encoded,
    "percent-encoded sequences present",
    UrlDecode,
    Arguments::from([("treat_plus_as_space".into(), ArgumentValue::Boolean(true))]),
    id: "encoding.url.decode@1",
    alias: "URL Decode",
    summary: "treat_plus_as_space=true",
    fragment: r#"{"op":"URL Decode","args":[true]}"#
);

text_probe!(
    probe_html,
    looks_like_html_entities,
    "HTML entities present",
    FromHtmlEntity,
    Arguments::new(),
    id: "encoding.html.decode@1",
    alias: "From HTML Entity",
    summary: "(none)",
    fragment: r#"{"op":"From HTML Entity","args":[]}"#
);

text_probe!(
    probe_defang,
    looks_defanged,
    "defanged URL markers (hxxp / [.] / [://])",
    FangUrl,
    Arguments::from([
        ("restore_dots".into(), ArgumentValue::Boolean(true)),
        ("restore_hxxp".into(), ArgumentValue::Boolean(true)),
        ("restore_slashes".into(), ArgumentValue::Boolean(true)),
    ]),
    id: "defang.fang_url@1",
    alias: "Fang URL",
    summary: "restore dots/hxxp/slashes",
    fragment: r#"{"op":"Fang URL","args":[true,true,true]}"#
);
