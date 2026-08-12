use ferrosift_core::OperationError;

use crate::failure::failed;

/// Resolves a `CyberChef` delimiter token into its literal separator text.
///
/// Mirrors `Utils.charRep` for the sequence delimiters used by the built-in
/// radix operations. Unknown tokens report the caller's stable failure code.
pub(crate) fn char_rep(token: &str, code: &'static str) -> Result<&'static str, OperationError> {
    match token {
        "Space" => Ok(" "),
        "Percent" => Ok("%"),
        "Comma" => Ok(","),
        "Semi-colon" => Ok(";"),
        "Colon" => Ok(":"),
        "Tab" => Ok("\t"),
        "Line feed" => Ok("\n"),
        "CRLF" => Ok("\r\n"),
        "Forward slash" => Ok("/"),
        "Backslash" => Ok("\\"),
        "Nothing (separate chars)" | "None" => Ok(""),
        _ => Err(failed(code)),
    }
}

/// Reports whether a character belongs to the JavaScript `\s` regex class.
///
/// `CyberChef` strips delimiters with `\s`-based regular expressions, so the
/// exact ECMAScript whitespace set (including the BOM) is load-bearing.
pub(crate) const fn is_js_whitespace(value: char) -> bool {
    matches!(
        value,
        '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}
