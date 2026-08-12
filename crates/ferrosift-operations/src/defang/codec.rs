//! Defang / fang transforms for IPs and URLs.

use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};
use regex_automata::{meta::Regex, util::syntax};

use crate::failure::failed;

const INVALID: &str = "defang.invalid_pattern";

pub(super) fn defang_ip(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let ipv4 = compile(
        r"(?:(?:\d|[01]?\d\d|2[0-4]\d|25[0-5])\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d|\d)(?:/\d{1,2})?",
    )?;
    let mut output = replace_all(input, &ipv4, |value| value.replace('.', "[.]"));
    output = defang_ipv6_colons(&output);
    ensure_len(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

fn defang_ipv6_colons(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut current = String::new();
    let flush = |buf: &mut String, out: &mut String| {
        if buf.contains(':') && buf.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
            out.push_str(&buf.replace(':', "[:]"));
        } else {
            out.push_str(buf);
        }
        buf.clear();
    };
    for ch in input.chars() {
        if ch.is_ascii_hexdigit() || ch == ':' {
            current.push(ch);
        } else {
            flush(&mut current, &mut output);
            output.push(ch);
        }
    }
    flush(&mut current, &mut output);
    output
}

pub(super) fn defang_url(
    input: &str,
    dots: bool,
    http: bool,
    slashes: bool,
    process: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::from(input);
    match process {
        "Valid domains and full URLs" => {
            output = replace_urls_and_domains(&output, dots, http, slashes)?;
        }
        "Only full URLs" => {
            output = replace_urls_only(&output, dots, http, slashes)?;
        }
        "Everything" => {
            output = apply_defang(&output, dots, http, slashes);
        }
        _ => return Err(failed(INVALID)),
    }
    ensure_len(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

pub(super) fn fang_url(
    input: &str,
    dots: bool,
    http: bool,
    slashes: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut output = String::from(input);
    if dots {
        output = output.replace("[.]", ".");
    }
    if http {
        output = output.replace("hxxp", "http");
    }
    if slashes {
        output = output.replace("[://]", "://");
    }
    ensure_len(&output, context)?;
    context.ensure_active()?;
    Ok(output)
}

fn replace_urls_only(
    input: &str,
    dots: bool,
    http: bool,
    slashes: bool,
) -> Result<String, OperationError> {
    const URL: &str = r#"(?i)[A-Z]+://[-\w]+(?:\.\w[-\w]*)+(?::\d+)?(?:/[^.!,?"<>\[\]{}\s\x7F-\u{00FF}]*(?:[.!,?]+[^.!,?"<>\[\]{}\s\x7F-\u{00FF}]+)*)?"#;
    let url = compile(URL)?;
    Ok(replace_all(input, &url, |value| {
        apply_defang(value, dots, http, slashes)
    }))
}

fn replace_urls_and_domains(
    input: &str,
    dots: bool,
    http: bool,
    slashes: bool,
) -> Result<String, OperationError> {
    let mut output = replace_urls_only(input, dots, http, slashes)?;
    let domain =
        compile(r"(?i)\b((?=[a-z0-9-]{1,63}\.)(xn--)?[a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,63}\b")?;
    output = replace_all(&output, &domain, |value| {
        apply_defang(value, dots, http, slashes)
    });
    Ok(output)
}

fn apply_defang(value: &str, dots: bool, http: bool, slashes: bool) -> String {
    let mut output = String::from(value);
    if dots {
        output = output.replace('.', "[.]");
    }
    if http {
        // Case-insensitive http -> hxxp
        let lower = output.to_ascii_lowercase();
        let mut rebuilt = String::with_capacity(output.len());
        let bytes = output.as_bytes();
        let mut index = 0_usize;
        while index < bytes.len() {
            if lower
                .get(index..)
                .is_some_and(|tail| tail.starts_with("http"))
            {
                rebuilt.push_str("hxxp");
                index += 4;
            } else {
                rebuilt.push(char::from(bytes[index]));
                index += 1;
            }
        }
        output = rebuilt;
    }
    if slashes {
        output = output.replace("://", "[://]");
    }
    output
}

fn compile(pattern: &str) -> Result<Regex, OperationError> {
    Regex::builder()
        .syntax(syntax::Config::new().unicode(true).utf8(true))
        .build(pattern)
        .map_err(|_| failed(INVALID))
}

fn replace_all(input: &str, regex: &Regex, map: impl Fn(&str) -> String) -> String {
    let mut output = String::with_capacity(input.len());
    let mut last = 0_usize;
    for found in regex.find_iter(input.as_bytes()) {
        if !input.is_char_boundary(found.start()) || !input.is_char_boundary(found.end()) {
            continue;
        }
        output.push_str(&input[last..found.start()]);
        output.push_str(&map(&input[found.start()..found.end()]));
        last = found.end();
    }
    output.push_str(&input[last..]);
    output
}

fn ensure_len(text: &str, context: &OperationContext<'_>) -> Result<(), OperationError> {
    if u64::try_from(text.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        Err(OperationError::OutputLimitExceeded)
    } else {
        Ok(())
    }
}
