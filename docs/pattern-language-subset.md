# Pattern language subset

`ferrosift-pattern` reads a hex-pattern source file, produces a checked
declaration tree describing binary structures, and evaluates that tree against
bytes into a value tree where every node carries its absolute offset and byte
size — enough to annotate a hex view without re-deriving the layout.

## Compatibility status

**No upstream compatibility is claimed yet.** FerroSift's rule is that a
compatibility claim must be backed by a pinned differential corpus, the way
[CyberChef 11.3.0 compatibility](compatibility/cyberchef-v11.3.0.md) is backed
by 583 pinned cases. No such corpus exists for a pattern-language runtime in
this repository, so this crate documents *its own* grammar and says nothing
about matching any other implementation. The claim will be made only once the
evidence exists, and this page will then state the pinned reference and case
count exactly as the CyberChef ledger does.

## Supported grammar

```text
pattern     := declaration*
declaration := struct | enum | bitfield | alias | placement

struct      := 'struct' IDENT '{' field* '}' ';'?
field       := type IDENT ('[' INT ']')? ';'

enum        := 'enum' IDENT ':' type '{' entry (',' entry)* ','? '}' ';'?
entry       := IDENT ('=' INT)?

bitfield    := 'bitfield' IDENT '{' member* '}' ';'?
member      := IDENT ':' INT ';'

alias       := 'using' IDENT '=' type ';'
placement   := type IDENT ('[' INT ']')? '@' INT ';'

type        := ('be' | 'le')? (BUILTIN | IDENT)
```

An enum entry without `= value` continues from the previous entry, starting
at zero. Bitfield member widths are 1 to 64 bits. Array lengths must be
positive. A name may be declared only once per pattern.

## Built-in types

| Category | Types |
|---|---|
| Unsigned | `u8`, `u16`, `u24`, `u32`, `u48`, `u64`, `u96`, `u128` |
| Signed | `s8`, `s16`, `s24`, `s32`, `s48`, `s64`, `s96`, `s128` |
| Floating point | `float` (4 bytes), `double` (8 bytes) |
| Other | `bool`, `char` (1 byte), `char16` (2 bytes) |

## Literals and comments

Integers accept decimal, `0x` hexadecimal, `0b` binary, and `0o` octal, with
`_` permitted as a digit separator. Character and string literals support the
`\n`, `\r`, `\t`, `\0`, `\\`, `\'`, and `\"` escapes. Both `//` line comments
and `/* */` block comments are ignored.

## Evaluation semantics

Evaluation walks each placement in source order and reads from the placement's
absolute address.

- **Endianness.** Types with no `be` / `le` prefix use the default in
  `EvalOptions`, which is little-endian. A prefix on a struct or alias *use*
  propagates to members that do not carry a prefix of their own; a member's
  own prefix always wins.
- **Layout.** Struct fields and array elements are laid out consecutively with
  no padding. A composite node's size is the span from its first byte to the
  end of its last child.
- **Signed integers** are sign-extended from their declared width.
- **Enums** read their backing type and resolve the value against the declared
  constants; an unmatched value is preserved with no name rather than failing.
- **Bitfields** occupy `ceil(total_bits / 8)` bytes and unpack members
  most-significant-bit first from a big-endian view of that span. This is the
  layout *this crate defines*, not a claim about another implementation.
- **Bounds.** Every read is checked against the real buffer length, so a
  pattern can never observe bytes that are not there.
- **Budgets.** Array lengths and nesting come from untrusted text, so
  `EvalOptions` caps the total node count (default 1,000,000) and the type
  nesting depth (default 64). Exceeding either fails with a stable code
  instead of exhausting memory or the stack.

## Not implemented

Each of these is a named future step, never a silent gap: functions,
`if` / `else`, loops, `while`-sized and unbounded arrays, pointers, unions,
namespaces, attributes, the preprocessor (`#include`, `#define`, `#pragma`),
and expressions beyond integer literals. Sources using them are rejected with
a stable code, never partially accepted.

## Failure codes

Every failure carries a stable code and a one-based source position. Codes
are matchable identifiers whose meaning does not change between releases.

| Code | Meaning |
|---|---|
| `pattern.lex.unterminated_comment` | Block comment is never closed |
| `pattern.lex.unterminated_text` | Character or string literal is never closed |
| `pattern.lex.invalid_escape` | Unsupported or truncated escape sequence |
| `pattern.lex.invalid_number` | Number has no digits, or a digit outside its radix |
| `pattern.lex.number_overflow` | Integer literal exceeds 128 bits |
| `pattern.lex.unexpected_character` | Character outside the supported subset |
| `pattern.parse.unexpected_token` | A body is never closed |
| `pattern.parse.expected_identifier` | A name was required |
| `pattern.parse.expected_symbol` | Required punctuation was missing |
| `pattern.parse.expected_integer` | An integer literal was required |
| `pattern.parse.expected_type` | A type name was required |
| `pattern.parse.invalid_array_length` | Array length is not positive |
| `pattern.parse.invalid_bit_width` | Bitfield width is outside 1..=64 |
| `pattern.parse.duplicate_declaration` | A name is declared more than once |
| `pattern.eval.out_of_bounds` | A read extends past the end of the data |
| `pattern.eval.unknown_type` | A referenced type is not declared in the pattern |
| `pattern.eval.depth_exceeded` | Type nesting exceeds the configured depth |
| `pattern.eval.node_budget_exceeded` | The value tree exceeds the configured node budget |
