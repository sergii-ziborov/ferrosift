# The value model

FerroSift carries a value between recipe steps as one of a small set of kinds.
The reference carries one as a *Dish*, which has more kinds than this does --
and, more importantly, defines a conversion for every pair of them. Where the
two disagree, a recipe that runs in both produces different bytes from the
second step onwards.

This page records what each conversion is, how it was checked, and what still
disagrees.

## The kinds, side by side

| Reference dish | FerroSift kind | Status |
|---|---|---|
| `byteArray`, `ArrayBuffer` | `Bytes` | matches |
| `string` | `Text` | matches |
| `number` | `Number`, and `Integer` where the value is whole | **done** |
| `html` | `Markup` | **done** |
| `JSON` | `Structured` | **done**, with one named limit below |
| `BigNumber` | `Decimal` | **done** — all three renderings, both readings, the arithmetic; 13 of the 16 blocked operations ported, plus `To Base` |
| `File`, `List<File>` | `Files` | matches |

Ten shipped operations used to declare `Text` where the reference declared
something else. All ten now declare their own kind: six carry a number, three
carry markup, and one carries a structure.

## Why the label is not cosmetic

A dish does not convert to bytes by printing itself. Each kind has its own
conversion, and two of them *lose information on purpose*:

- **`html` to anything** runs `unescapeHtml(stripHtmlTags(value, true))`. The
  next operation therefore sees the text with the markup removed and the
  entities resolved -- not the markup FerroSift currently passes on.
- **`JSON` to anything** runs `JSON.stringify(value, null, 4)`, so the four
  spaces are part of the bytes rather than a display choice.
- **`number` to anything** prints the number the way JavaScript prints it,
  which is not how every language prints one.

The terminal step is not where this shows. A fixture that pins the last
output of a one-step recipe passes either way, which is why the gap survived
ten operations. It shows on the *second* step.

## How the gap was found

The corpus harness reads an HTML operation's own dish value so that the markup
can be pinned at all -- before that, asking for its bytes returned the markup
with the markup taken out. Making that work exposed the other half: FerroSift
hands the next operation the markup, and the reference hands it the stripped
text.

The harness was given a rule -- *an HTML operation may only end a recipe* --
which guarded against pinning a divergence rather than fixing one. That rule
is gone: `Markup` exists, the conversion runs at the step boundary, and the
chained recipe is pinned instead of refused.

## The order this is being closed in

Markup first, because it is the only one where a chained recipe is *provably*
wrong today and the proof is a corpus case rather than an argument.

1. ~~**Markup**~~ -- **done**. Three operations declare it, and
   `offset_chain_upper_23` pins a recipe whose second step receives `ABCDEF`
   where the first produced `<span class='hl5'>abc</span>d...`. Passing the
   tags forward would have produced `<SPAN class='hl5'>ABC</SPAN>` instead --
   different bytes at every later step.
2. ~~**Number**~~ -- **done**. Chi Square and Index of Coincidence carry the
   number itself; the four whose answer is a count keep `Integer`, which
   converts through the same rendering. FerroSift therefore has two numeric
   kinds where the reference has one, and that is deliberate: a caller gets a
   count as a count rather than having to parse one back out of a string, and
   the conversion table is what keeps the printed bytes identical.
3. ~~**Structured as the JSON dish**~~ -- **done**. The projection renders
   `JSON.stringify(value, null, 4)`, so the four spaces are bytes rather than a
   display choice, and Parse TLV carries the structure rather than a string it
   built by hand.
4. ~~**Decimal**~~ -- **done**, and now carrying operations. The kind, the
   canonical form, all three renderings, both readings, the arithmetic and the
   base conversions are pinned against the real library. Thirteen of the
   sixteen operations that were blocked on it are ported, and so is `To Base`,
   which the dependency scan had missed because it never imported the library
   — it was handed a number by the dish. The three that remain each wait on a
   second thing as well as on this.

   Two of the thirteen turned out not to need the arithmetic at all. `To BCD`
   and `From BCD` never add anything: they read the digits the dish already
   rendered and hand digits back. What blocked them was the value *kind*, not
   the library behind it, so they ship with no feature gate.

## What `Decimal` is

A canonical representation rather than a dependency, so `ferrosift-model` stays
free of an arbitrary-precision crate and the arithmetic backend is the only
thing that has to agree with one:

```text
sign, coefficient (digits), exponent10, and an optional NaN / Infinity
```

That is enough to render exactly what the reference renders, and enough for a
backend to load into whatever it uses. Putting a crate in the model instead
would make every consumer of a value depend on a choice only arithmetic cares
about.

### Three renderings, not one

The reference writes a `BigNumber` three different ways, and a recipe can
reach all three:

| Method | Exponential notation | Where it is used |
|---|---|---|
| `toFixed()` | never | the dish, so every operation that *hands back* a number |
| `toString()` | at or below `1e-7`, at or above `1e21` | an operation that joins numbers into a string itself |
| `toString(base)` | never, not even for base ten | To Base, and the hexadecimal filetime formats |

`DecimalValue::to_fixed`, `DecimalValue::to_notation`, and
`jscompat::bignumber::to_base` are those three. MOD needs the second: it joins
its remainders with `Array.prototype.join`, which calls `toString`. A port
carrying only `to_fixed` would be right about a remainder of `2.5` and wrong
about a remainder of `1e-8` — in an operation whose other answers all looked
correct.

The `toString()` threshold is `21`, read from the library rather than from its
documentation, which says `20`. That is the same kind of error as the exponent
range, where the documentation says a billion and the code says ten million.
Both are recorded in `tests/fixtures/decimal.json` along with the cases either
side of them.

`toString(base)` rounds differently from everything else here, and the
difference only shows on an odd base. It decides from the twenty-first digit
alone, compared against half the base as a real number — and no digit of an
odd base is worth exactly half, so an exact tie **truncates**. A tenth in base
five repeats as `0.0222…`, sits exactly half a place above the twentieth
digit, and comes out truncated where the same tie in base sixteen rounds away
from zero. A sweep of one fraction per base is what found it; a sample would
have missed it.

### Reading is not the mirror of writing

Two constructors, with rules that run *backwards* from each other in four
places. Both are pinned, because deriving either from the other would be
wrong:

| | `new BigNumber(text)` | `new BigNumber(text, base)` |
|---|---|---|
| empty string | refused | zero |
| `NaN`, `Infinity` | read | refused |
| `0x` prefix | read as hexadecimal | refused |
| `e` | an exponent marker | a digit |
| letter case | irrelevant | must not mix within one value |

`DecimalValue::parse` and `jscompat::bignumber::parse_in_base` are those two.
There is a third reading, `DecimalValue::read`, which differs from `parse` in
one respect: it *refuses* what the constructor refuses instead of answering
not-a-number. A dish catches the constructor's exception and substitutes
not-a-number; an operation that calls the constructor itself sees the
exception and stops the recipe. The text `NaN` is a value either way, and the
text `abc` is a value to one and an error to the other.

## The mechanism, now built

Conversion goes through a canonical byte form, because that is what the
reference does: `Dish._translate` writes the value to an `ArrayBuffer` and
reads it back as the target type, whatever the pair. `Value::into_dish_bytes`
and `Value::from_dish_bytes` are those two halves, and `reinterpret` is their
composition.

A table of ordered pairs was the first attempt and was wrong in shape. Ten
kinds make ninety pairs, every one of them a chance to disagree with the
reference in a direction nobody chained -- and `Number` to `Decimal` was
already missing from it. `ValueKind::converts_to` now asks only whether one
end can be written and the other read, so it cannot promise a conversion
execution declines to perform. A test asserts that agreement across every
kind and every target.

Two kinds are writable but not readable, and the asymmetry is deliberate.
`Integer` read back from digits would have to decide what to do with a
fraction the reference would have kept; `Structured` would need a JSON parser
obliged to agree with `JSON.parse`. Both are absent rather than approximated.

### How the rendering is checked

`tools/cyberchef-oracle/decimal.mjs` asks the real `bignumber.js` inside the
pinned checkout what it renders for thirty-one inputs, and
`crates/ferrosift-model/tests/decimal.rs` replays every answer. Two things came
out of that which reading the documentation would not have given:

- The dish converts with **`toFixed()`**, not `toString()`. `toFixed` never
  uses exponential notation, so `1e+25` is written out in full. Reproducing
  `toString` would have been wrong in exactly the cases hardest to notice.
- The constructor **throws** on input it cannot read. It does not return
  not-a-number, which is what the documentation's talk of NaN values suggests.
  The dish catches and substitutes, so what a recipe observes is the
  substitution -- and that is what `DecimalValue::parse` reproduces.

## Key order in the JSON projection

`StructuredValue::Object` holds its members in the order they were added, and
`StructuredValue::enumeration_order` applies JavaScript's rule on top: keys that
look like array indices come first in numeric order, and every other key follows
in insertion order. That is what `JSON.stringify` writes and what a later step
reading the object sees.

It was a **sorted** map, and this page recorded the divergence that produced:
the two agreed only where a value's keys happened to sort into the order they
were added, which they did for Parse TLV — `key`, `length`, `value` is both —
so nothing shipped was wrong and the next operation with unsorted keys would
have been. The map was replaced rather than the limitation lived with, and
`crates/ferrosift-model/tests/dish.rs` pins the enumeration against cases a
sorted map answers wrongly: `10` before `2`, `a` before `b`.

The paragraph that described the old behaviour outlived it by several
revisions, here and in the model's own doc comment. That is the ordinary way a
rationale goes stale — the code moved and the sentence explaining why it was
that way did not — and it is why the numbers on these pages are generated
wherever they can be.

## What the rewrite found

Text became bytes as UTF-8, and the reference does not do that. It writes one
byte per UTF-16 code unit and falls back to UTF-8 only when a unit exceeds 255
-- so `é` is the single byte `0xE9` there and was the pair `0xC3 0xA9` here.

No fixture caught it, and none would have: a corpus of recipes reaches a
conversion only when two steps happen to form that pair, so a conversion
nobody chained is a conversion nobody checked. `crates/ferrosift-model/tests/dish.rs`
tests the conversions directly for that reason.
