//! Pattern-language front end for `FerroSift` binary structure parsing.
//!
//! This crate reads a hex-pattern source file, produces a checked declaration
//! tree, and evaluates it against bytes into a value tree carrying the exact
//! offset and size of every field. It implements the **subset** documented in
//! `docs/pattern-language-subset.md`: structs, enums, bitfields, `using`
//! aliases, fixed-size arrays, endianness prefixes, and absolute placements.
//!
//! Compatibility with any upstream pattern-language runtime is **not yet
//! claimed**. `FerroSift` only claims compatibility that a pinned differential
//! corpus can demonstrate, and no such corpus exists for this language yet.
//!
//! ```
//! use ferrosift_pattern::{EvalOptions, NodeValue};
//!
//! let pattern = ferrosift_pattern::parse(
//!     "struct Header { be u16 magic; u8 version; };
//!      Header header @ 0x00;",
//! )?;
//! let nodes = ferrosift_pattern::evaluate(
//!     &pattern,
//!     &[0xca, 0xfe, 0x03],
//!     &EvalOptions::default(),
//! )?;
//!
//! let magic = nodes[0].child("magic").expect("field");
//! assert_eq!(magic.value, NodeValue::Unsigned(0xcafe));
//! assert_eq!((magic.offset, magic.size), (0, 2));
//! # Ok::<(), ferrosift_pattern::PatternError>(())
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;

pub use ast::{
    AliasDeclaration, BitfieldDeclaration, BitfieldMember, Builtin, Declaration, Endian,
    EnumDeclaration, EnumEntry, Field, Pattern, Placement, StructDeclaration, TypeKind,
    TypeReference,
};
pub use error::{PatternError, Position};
pub use eval::{EvalOptions, Node, NodeValue, evaluate};
pub use parser::parse;
