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
| `BigNumber` | `Decimal` | kind and rendering **done**; the 16 operations still need porting |
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
4. ~~**Decimal**~~ -- the kind, the canonical form, and the rendering are
   **done** and pinned against the real library. The sixteen operations that
   need it are still unported: what is finished is the representation they
   would land in, not the operations.

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

## The mechanism, now built

`Value::reinterpret` performs The dish's own conversion, ``ValueKind::converts_to`is the single table both it and preflight read, and the executor applies it
before handing a value to an operation. Preflight accepts a pair when a
conversion exists rather than only when the kinds already match.

The projection layer, not the extra variants, was the substance of this work.
What remains is to route the kinds that still travel as `Text` through it.

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

## The one limit left in the JSON projection

`StructuredValue::Object` is a **sorted** map, and `JSON.stringify` writes keys
in **insertion order**. The two agree only where a value's keys happen to sort
into the order they were added.

They do for Parse TLV -- `key`, `length`, `value` is both -- so nothing shipped
today is wrong. An operation whose keys do not sort that way would need an
order-preserving map before it could claim compatibility, and that is a change
to the model rather than to the operation.

This is recorded rather than fixed because no operation needs it yet, and
because a limitation nobody has written down is the kind that gets discovered
by a user.
