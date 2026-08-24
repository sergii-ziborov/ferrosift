//! Pattern-language front end for `FerroSift` binary structure parsing.
//!
//! This crate reads a hex-pattern source file and produces a checked
//! declaration tree. It implements the **subset** documented in
//! `docs/pattern-language-subset.md`: structs, enums, bitfields, `using`
//! aliases, fixed-size arrays, endianness prefixes, and absolute placements.
//!
//! Compatibility with any upstream pattern-language runtime is **not yet
//! claimed**. `FerroSift` only claims compatibility that a pinned differential
//! corpus can demonstrate, and no such corpus exists for this language yet.
//!
//! ```
//! let pattern = ferrosift_pattern::parse("struct Header { u32 magic; };")?;
//! assert_eq!(pattern.declarations.len(), 1);
//! # Ok::<(), ferrosift_pattern::PatternError>(())
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod ast;
mod error;
mod lexer;
mod parser;

pub use ast::{
    AliasDeclaration, BitfieldDeclaration, BitfieldMember, Builtin, Declaration, Endian,
    EnumDeclaration, EnumEntry, Field, Pattern, Placement, StructDeclaration, TypeKind,
    TypeReference,
};
pub use error::{PatternError, Position};
pub use parser::parse;
