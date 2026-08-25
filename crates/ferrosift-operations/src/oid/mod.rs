//! ASN.1 object identifiers.
//!
//! An OID names a thing — a signature algorithm, a certificate extension, a
//! company — and DER packs it into bytes with two different rules: the first
//! two arcs share one value, everything after is base-128 with continuation
//! bits. Both directions here reproduce the reference's ASN.1 library, quirks
//! included, because a certificate parser reading these bytes is not consulting
//! the standard, it is consulting whatever wrote them.
//!
//! ## Two deliberate divergences
//!
//! The reference reaches for a bignum whose parser skips characters it does not
//! recognise, and both operations feed it text that can contain such
//! characters. Where they do, the reference returns a number derived from the
//! letters of the word `NaN`:
//!
//! - `Object Identifier to Hex` on `"1"` returns the literal string `"NaN"`,
//!   because a missing second arc makes the combined first pair not-a-number.
//!   `"1..2"` returns `"NaN02"`.
//! - `Hex to Object Identifier` on `"2azz"` returns `"1.2.95"`, where 95 comes
//!   from reading the characters `N`, `a`, `N` as bignum digits.
//!
//! `FerroSift` refuses both instead. This is a divergence and is recorded as one
//! in `docs/compatibility/cyberchef-v11.3.0.md` — it is not compatibility, and
//! calling it that would be worse than declining. The reasoning is that no
//! caller wants `95` for input `zz`, and an operation that answers a question
//! it cannot answer is more dangerous than one that says so. Reproducing it
//! would also mean reproducing a specific bignum's digit table and word-size
//! carry behaviour, which would be pinned to that library rather than to any
//! specification.
//!
//! Everything reachable from well-formed input matches byte for byte, including
//! the encoder's genuine bug for first pairs above 255 — `2.999` produces three
//! hex digits — which is reproduced because well-formed input reaches it.

mod codec;
mod operation;

pub use operation::{HexToObjectIdentifier, ObjectIdentifierToHex};
