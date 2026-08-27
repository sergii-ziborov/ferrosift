//! Pattern-language front end for `FerroSift` binary structure parsing.
//!
//! This crate reads a hex-pattern source file, produces a checked declaration
//! tree, and evaluates it against bytes into a value tree carrying the exact
//! offset and size of every field. It implements the **subset** documented in
//! `docs/pattern-language-subset.md`: structs, unions, enums, bitfields,
//! `using` aliases, counted and `while` arrays, `if`/`else`, padding,
//! endianness prefixes, expressions with `sizeof` and `$`, and placements.
//!
//! # Compatibility
//!
//! Measured against `ImHex`'s own runtime rather than asserted. `plcli` is
//! built from a pinned checkout of `WerWolv/PatternLanguage`, answers 104 cases
//! covering one construct each, and
//! `crates/ferrosift-pattern/tests/differential.rs` replays every one of them.
//!
//! **102 of the 104 agree.** The other two ask for `sizeof` of a declared type,
//! which this crate does not compute; they are held in the fixture and asserted
//! to fail, so the day that changes the test says so. Nothing is skipped: a
//! case answered differently fails the replay.
//!
//! What that does *not* say is how much of the real `.hexpat` ecosystem parses
//! here — the corpus separates constructs rather than sampling patterns people
//! wrote. `docs/pattern-language-subset.md` has the grammar, the case list, and
//! what is still missing.
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
    AliasDeclaration, ArrayLength, BinaryOperator, BitfieldDeclaration, BitfieldMember, Builtin,
    Declaration, Endian, EnumDeclaration, EnumEntry, Expression, Field, Member, Pattern, Placement,
    SizeOfTarget, StructDeclaration, TypeKind, TypeReference, UnaryOperator, UnionDeclaration,
};
pub use error::{PatternError, Position};
pub use eval::{EvalOptions, Node, NodeValue, evaluate};
pub use parser::parse;
