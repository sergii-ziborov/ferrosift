//! Tag stripping and smart-character folding.

mod codec;
mod operation;

pub use operation::{EscapeSmartCharacters, StripHtmlTags};
