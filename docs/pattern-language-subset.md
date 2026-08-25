# Pattern language subset

`ferrosift-pattern` reads a hex-pattern source file, produces a checked
declaration tree describing binary structures, and evaluates that tree against
bytes into a value tree where every node carries its absolute offset and byte
size — enough to annotate a hex view without re-deriving the layout.

## Where this grammar comes from

The syntax is modelled on the **ImHex pattern language** (`.hexpat`) — the
same `struct` / `union` / `enum` / `bitfield` / `using` declarations, the same
`be` and `le` prefixes, the same `Type name @ address;` placement, and the
same `u8`…`u128` type names.

Recording that here is itself a fix. Until now nothing in this repository
named the language it follows, in the docs, the README, or a single comment,
which left every grammar decision looking arbitrary rather than inherited.

## Compatibility status

**No upstream compatibility is claimed.** FerroSift's rule is that a
compatibility claim must be backed by a pinned differential corpus, the way
[CyberChef 11.3.0 compatibility](compatibility/cyberchef-v11.3.0.md) is backed
by its pinned cases. No pattern-language runtime is vendored in this
repository and no such corpus exists, so this page documents *what this crate
does* and nothing more. Naming the inspiration above is not a claim of
agreement with it: where this crate has had to decide something the grammar
alone does not settle — bitfield bit order is the clearest case — the choice
is marked as this crate's own.

The claim will be made only once the evidence exists, and this page will then
state the pinned reference and case count exactly as the CyberChef ledger
does.

## Supported grammar

```text
pattern     := declaration*
declaration := struct | union | enum | bitfield | alias | placement

struct      := 'struct' IDENT body ';'?
union       := 'union' IDENT body ';'?
body        := '{' member* '}'
member      := field | conditional | padding
field       := type IDENT array? ';'
conditional := 'if' '(' expr ')' body ('else' (conditional | body))?
padding     := 'padding' '[' expr ']' ';'
array       := '[' expr ']' | '[' 'while' '(' expr ')' ']'

enum        := 'enum' IDENT ':' type '{' entry (',' entry)* ','? '}' ';'?
entry       := IDENT ('=' expr)?

bitfield    := 'bitfield' IDENT '{' bits* '}' ';'?
bits        := IDENT ':' expr ';'

alias       := 'using' IDENT '=' type ';'
placement   := type IDENT array? '@' expr ';'

type        := ('be' | 'le')? (BUILTIN | IDENT)

expr        := ternary
ternary     := binary ('?' ternary ':' ternary)?
binary      := unary (OP binary)*
unary       := ('-' | '~' | '!')? primary
primary     := INT | CHAR | 'true' | 'false' | '$'
             | 'sizeof' '(' (BUILTIN | path) ')'
             | path | '(' expr ')'
path        := IDENT ('.' IDENT)*
```

An enum entry without `= value` continues from the previous entry, starting
at zero. Bitfield member widths are 1 to 64 bits. A literal array length must
be positive. A name may be declared only once per pattern.

## Expressions

Anywhere an integer was once required, an expression is accepted. This is what
lets one pattern describe a format rather than one file.

| Position | Sees |
|---|---|
| Array length, `[expr]` | Fields already read in the same body |
| Array test, `[while(expr)]` | The same, with `$` at the next element |
| Placement address, `@ expr` | Placements already evaluated |
| Conditional test, `if (expr)` | Fields already read in the same body |
| `padding[expr]` | The same |
| Enum value, bit width | Nothing — folded once while parsing |

Operators are C's, with C's precedence: `* / %`, then `+ -`, `<< >>`,
`< <= > >=`, `== !=`, `&`, `^`, `|`, `&&`, `||`, and the `?:` conditional.
Prefix `-`, `~`, and `!` bind tighter than all of them. Comparisons yield 0 or
1 and any non-zero value is true, so a test and a count are the same kind of
thing.

`&&`, `||`, and `?:` evaluate only what they must, which is what makes
`n == 0 ? 1 : 4 / n` safe to write. Arithmetic is checked: an overflow, a
division by zero, or a shift of 128 or more is a stable failure code rather
than a wrapped value.

`$` is the offset the current field starts at. `sizeof(u32)` is a built-in's
width; `sizeof(field)` is the span a field actually occupied, which is the
only way to ask the size of something whose length varied.

A field may only refer to fields declared **before** it. That is not a
restriction this crate adds — a later field's bytes have not been read, so its
value does not exist yet.

Two limits here *are* this crate's, and are worth knowing before writing a
pattern against it:

- **A nested type cannot see the body that holds it.** Expressions resolve
  against siblings only, so `struct Inner { u8 data[parent.length]; }` has no
  way to reach `length`. Passing the value down as a field of the inner type
  is not possible either, because there are no parameters yet. Where a real
  format needs this, the inner fields have to be written into the outer body.
- **`sizeof` does not take a named type.** `sizeof(u32)` works and
  `sizeof(some_field)` works, but `sizeof(Header)` is read as a field path and
  fails with `unknown_field`. A declared type's width is not always a
  constant — a body with an `if` in it has no single size — so answering it
  properly means evaluating the type, which is a larger change than this.

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
- **Unions.** Every member is read from the union's own offset, and the size
  is the widest member rather than the sum. Members still see the ones
  declared before them, so a union can be discriminated by a field read
  earlier.
- **Conditionals.** `if` contributes its members to the enclosing body rather
  than producing a node of its own, so a condition changes which fields exist
  without changing the shape of the value tree. `else if` nests, and exactly
  one arm is taken.
- **Padding.** `padding[n]` advances the cursor by `n` bytes and produces no
  node. The bytes it covers are still counted in the enclosing size.
- **`while` arrays.** Elements are read while the test holds, with `$` bound
  to where the next element would start. An element of zero width would spin,
  and the node budget is what stops it — the same bound that stops a wrong
  count.
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

Each of these is a named future step, never a silent gap: functions and their
`return`, `while` and `for` statements, `match`, pointers (`Type *p : u32`),
namespaces, attributes (`[[color]]`, `[[name]]`, `[[hidden]]`, …), the
preprocessor (`#include`, `#define`, `#pragma`), `str` and `auto`, unbounded
arrays terminated by a sentinel, `in` / `out` variables, and the `parent` and
`this` scopes. Sources using them are rejected with a stable code, never
partially accepted.

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
| `pattern.eval.unknown_field` | An expression names a field not readable from there |
| `pattern.eval.not_a_number` | An expression uses a float or a composite as a number |
| `pattern.eval.arithmetic_overflow` | An expression overflows 128 bits, or shifts too far |
| `pattern.eval.divide_by_zero` | An expression divides or takes a remainder by zero |
| `pattern.eval.invalid_length` | A computed array or padding length is negative or too large |
| `pattern.eval.depth_exceeded` | Type nesting exceeds the configured depth |
| `pattern.eval.node_budget_exceeded` | The value tree exceeds the configured node budget |
