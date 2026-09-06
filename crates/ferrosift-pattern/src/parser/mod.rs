mod cursor;
mod expression;
mod grammar;
mod resolve;

use crate::ast::Pattern;
use crate::error::PatternError;
use crate::lexer;

pub use resolve::parse_with;

/// Parses pattern source into its declarations.
///
/// Single-source entry point: `import` / `#include` are refused with
/// `pattern.parse.unsupported_directive`. Use [`parse_with`] and a
/// [`crate::PatternResolver`] to expand them.
///
/// # Errors
///
/// Returns a [`PatternError`] with a stable `pattern.lex.*` or
/// `pattern.parse.*` code describing the first problem found, together with
/// the source position it was detected at.
pub fn parse(source: &str) -> Result<Pattern, PatternError> {
    let tokens = lexer::scan(source)?;
    let mut cursor = cursor::Cursor::new(tokens);
    grammar::pattern(&mut cursor)
}
