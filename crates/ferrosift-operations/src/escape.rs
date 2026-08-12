//! CyberChef-compatible escape parsing for Extended / binaryString args.

use alloc::string::String;

/// Mirrors `Utils.parseEscapedChars` for replace and Extended find strings.
pub(crate) fn parse_escaped_chars(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(value) = chars.next() {
        if value != '\\' {
            output.push(value);
            continue;
        }
        let Some(next) = chars.next() else {
            output.push('\\');
            break;
        };
        match next {
            '\\' => output.push('\\'),
            'a' => output.push('\u{0007}'),
            'b' => output.push('\u{0008}'),
            't' => output.push('\t'),
            'n' => output.push('\n'),
            'v' => output.push('\u{000B}'),
            'f' => output.push('\u{000C}'),
            'r' => output.push('\r'),
            '\'' | '"' => output.push(next),
            'x' => push_hex_escape(&mut output, &mut chars, 2, false),
            'u' => push_unicode_escape(&mut output, &mut chars),
            '0'..='7' => push_octal_escape(&mut output, &mut chars, next),
            other => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    output
}

fn push_hex_escape(
    output: &mut String,
    chars: &mut core::iter::Peekable<core::str::Chars<'_>>,
    width: usize,
    brace: bool,
) {
    let mut digits = String::new();
    while digits.len() < width {
        match chars.peek().copied() {
            Some(digit) if digit.is_ascii_hexdigit() => {
                digits.push(digit);
                chars.next();
            }
            _ => break,
        }
    }
    if brace {
        let _ = chars.next_if_eq(&'}');
    }
    if digits.len() == width
        && let Ok(code) = u32::from_str_radix(&digits, 16)
        && let Some(ch) = char::from_u32(if width == 2 { code & 0xff } else { code })
    {
        output.push(ch);
        return;
    }
    output.push(if width == 2 { 'x' } else { 'u' });
    if brace {
        output.push('{');
    }
    output.push_str(&digits);
}

fn push_unicode_escape(
    output: &mut String,
    chars: &mut core::iter::Peekable<core::str::Chars<'_>>,
) {
    if chars.peek() == Some(&'{') {
        chars.next();
        let mut digits = String::new();
        while let Some(digit) = chars.peek().copied() {
            if digit == '}' {
                chars.next();
                break;
            }
            if digit.is_ascii_hexdigit() && digits.len() < 6 {
                digits.push(digit);
                chars.next();
            } else {
                break;
            }
        }
        if let Ok(code) = u32::from_str_radix(&digits, 16)
            && let Some(ch) = char::from_u32(code)
        {
            output.push(ch);
            return;
        }
        output.push('u');
        output.push('{');
        output.push_str(&digits);
        return;
    }
    push_hex_escape(output, chars, 4, false);
}

fn push_octal_escape(
    output: &mut String,
    chars: &mut core::iter::Peekable<core::str::Chars<'_>>,
    first: char,
) {
    let mut digits = String::new();
    digits.push(first);
    for _ in 0..2 {
        match chars.peek().copied() {
            Some(digit @ '0'..='7') => {
                digits.push(digit);
                chars.next();
                if digits.len() == 3 {
                    break;
                }
            }
            _ => break,
        }
    }
    if let Ok(value) = u32::from_str_radix(&digits, 8)
        && let Some(ch) = char::from_u32(value & 0xff)
    {
        output.push(ch);
    }
}
