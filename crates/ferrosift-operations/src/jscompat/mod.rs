//! JavaScript semantics the reference relies on, modelled once.
//!
//! Porting a `CyberChef` operation is rarely about the algorithm. It is about
//! what `parseInt` does with `"0x1f "`, which characters `\s` covers, how a
//! lone surrogate survives a round trip, and what order an object hands back
//! its keys. Those questions have the same answer in every operation, and
//! answering them separately in each port is how two operations end up
//! disagreeing about the same language.
//!
//! Everything here is therefore shared, and pinned against Node directly
//! rather than only through the operations that use it — see
//! `tests/jscompat.rs`. An operation that needs one of these behaviours should
//! reach for this module rather than reimplement it, and a new behaviour
//! discovered while porting belongs here with a fixture beside it.
//!
//! What is modelled today:
//!
//! - [`number`] — `parseInt` prefix parsing, radix prefixes, and the byte
//!   coercion the reference's byte-array validation applies.
//! - [`string`] — conversions between JavaScript strings and byte arrays,
//!   including the UTF-16 code-unit view that decides astral behaviour.
//! - [`object`] — key ordering, which is insertion order except for
//!   integer-like keys, and is observable wherever the reference builds a map.
//! - [`escape`] — the escape sequences `Utils.parseEscapedChars` understands.
//! - [`delim`] — delimiter tokens and the JavaScript definition of
//!   whitespace, which is wider than `char::is_whitespace`.

pub(crate) mod delim;
pub(crate) mod escape;
pub(crate) mod number;
pub(crate) mod object;
pub(crate) mod string;
