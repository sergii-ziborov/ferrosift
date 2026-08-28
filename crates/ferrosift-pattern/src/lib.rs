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
//! # Bytes that are not a buffer
//!
//! [`evaluate`] takes a slice, which requires the whole subject in memory.
//! [`evaluate_with`] takes anything implementing [`ByteSource`], which does
//! not — and that is the case this engine is for. A disk image or a firmware
//! dump is routinely larger than the machine reading it, while a pattern
//! touches a vanishing fraction of it: one scalar per field, at a known
//! offset, of at most [`MAX_SCALAR_BYTES`]. The bounds check stays with the
//! evaluator, so an implementation is never asked for a range the pattern had
//! no right to.
//!
//! # Arrays of scalars
//!
//! An array whose element type resolves to a built-in — through any number of
//! `using` aliases — is kept as [`NodeValue::Scalars`]: the bytes it was read
//! from, decoded one element at a time by [`ScalarArray::get`]. One [`Node`]
//! per element is the obvious representation and the wrong one, because a
//! `Node` carries two heap strings and a `u8` carries one byte.
//!
//! Such a node has no [`children`](Node::children). [`Node::element_count`]
//! and [`Node::element`] read either representation, so a caller walking an
//! array does not have to ask which one it got. An array of structs, unions,
//! enums or bitfields keeps a [`NodeValue::Group`]: those elements are trees,
//! and there is nothing to defer.
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
pub use eval::{
    ByteSource, EvalOptions, MAX_SCALAR_BYTES, Node, NodeValue, ScalarArray, SourceError, evaluate,
    evaluate_with,
};
pub use parser::parse;
