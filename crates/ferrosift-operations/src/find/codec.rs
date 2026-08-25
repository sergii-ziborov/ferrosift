use alloc::string::String;

use ferrosift_core::{OperationContext, OperationError};
use regex_automata::{meta::Regex, util::syntax};

use crate::failure::failed;
use crate::jscompat::escape::parse_escaped_chars;

const INVALID_FIND: &str = "text.find_replace.invalid_pattern";
const INVALID_TYPE: &str = "text.find_replace.invalid_type";

/// Packed `CyberChef` match flags (`g`/`i`/`m`/`s`).
#[derive(Clone, Copy)]
pub(super) struct MatchFlags {
    bits: u8,
}

impl MatchFlags {
    pub(super) const GLOBAL: u8 = 0b0001;
    pub(super) const CASE_INSENSITIVE: u8 = 0b0010;
    pub(super) const MULTILINE: u8 = 0b0100;
    pub(super) const DOT_ALL: u8 = 0b1000;

    pub(super) const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    fn global(self) -> bool {
        self.bits & Self::GLOBAL != 0
    }

    fn case_insensitive(self) -> bool {
        self.bits & Self::CASE_INSENSITIVE != 0
    }

    fn multiline(self) -> bool {
        self.bits & Self::MULTILINE != 0
    }

    fn dot_matches_all(self) -> bool {
        self.bits & Self::DOT_ALL != 0
    }
}

pub(super) fn replace(
    input: &str,
    find_type: &str,
    find: &str,
    replace: &str,
    flags: MatchFlags,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let replace = parse_escaped_chars(replace);
    let pattern = match find_type {
        "Simple string" => escape_regex(find),
        value if value.starts_with("Extended") => escape_regex(&parse_escaped_chars(find)),
        "Regex" => String::from(find),
        _ => return Err(failed(INVALID_TYPE)),
    };
    let regex = Regex::builder()
        .syntax(
            syntax::Config::new()
                .case_insensitive(flags.case_insensitive())
                .multi_line(flags.multiline())
                .dot_matches_new_line(flags.dot_matches_all())
                .unicode(true)
                .utf8(true),
        )
        .build(&pattern)
        .map_err(|_| failed(INVALID_FIND))?;

    let mut output = String::with_capacity(input.len());
    let mut last = 0_usize;
    let mut count = 0_usize;
    for found in regex.find_iter(input.as_bytes()) {
        if count.is_multiple_of(256) {
            context.ensure_active()?;
        }
        let start = found.start();
        let end = found.end();
        if !input.is_char_boundary(start) || !input.is_char_boundary(end) {
            continue;
        }
        output.push_str(&input[last..start]);
        output.push_str(&replace);
        last = end;
        count += 1;
        if !flags.global() {
            break;
        }
    }
    output.push_str(&input[last..]);
    if u64::try_from(output.len()).map_or(true, |size| size > context.budget().max_output_bytes) {
        return Err(OperationError::OutputLimitExceeded);
    }
    context.ensure_active()?;
    Ok(output)
}

fn escape_regex(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(
            ch,
            '.' | '*'
                | '+'
                | '?'
                | '^'
                | '='
                | '!'
                | ':'
                | '$'
                | '{'
                | '}'
                | '('
                | ')'
                | '|'
                | '['
                | ']'
                | '/'
                | '\\'
        ) {
            output.push('\\');
        }
        output.push(ch);
    }
    output
}
