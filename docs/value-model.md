# The value model, and what it still owes the reference

FerroSift carries a value between recipe steps as one of a small set of kinds.
The reference carries one as a *Dish*, which has more kinds than this does --
and, more importantly, defines a conversion for every pair of them. Where the
two disagree, a recipe that runs in both produces different bytes from the
second step onwards.

This page records the disagreement that exists today, why it is not cosmetic,
and the order it is being closed in.

## The kinds, side by side

| Reference dish | FerroSift kind | Status |
|---|---|---|
| `byteArray`, `ArrayBuffer` | `Bytes` | matches |
| `string` | `Text` | matches |
| `number` | *none* -- reported as `Text` | **owed** |
| `html` | *none* -- reported as `Text` | **owed** |
| `JSON` | *none* -- reported as `Text` | **owed** |
| `BigNumber` | *none* | **owed**, and blocks 16 operations |
| `File`, `List<File>` | `Files` | matches |

Ten shipped operations already declare `Text` where the reference declares
something else: six `number` (Chi Square, Count occurrences, Index of
Coincidence, Levenshtein Distance, `MurmurHash3`, XKCD Random Number), three
`html` (Offset checker, Parse colour code, To Table), and one `JSON` (Parse
TLV).

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
and that rule is a placeholder for this page. It is a guard against pinning a
divergence, not a fix for one, and it should be deleted once `Markup` exists.

## The order this is being closed in

Markup first, because it is the only one where a chained recipe is *provably*
wrong today and the proof is a corpus case rather than an argument.

1. **`Markup`** -- three operations, and it retires the harness rule. The
   projection to text is the reference's strip-and-unescape, so a chained case
   becomes pinnable and proves the mechanism end to end.
2. **`Number`** -- six operations. The projection is the JavaScript rendering
   already written as `jscompat::float::to_js_string`.
3. **`Structured` as the JSON dish** -- one operation today. The kind exists;
   what is missing is the four-space projection.
4. **`Decimal`** -- no operations yet, and the largest prize: sixteen are
   blocked on `bignumber.js` alone.

## What `Decimal` should be

A canonical representation rather than a dependency, so `ferrosift-model`
stays free of an arbitrary-precision crate and the arithmetic backend is the
only thing that has to agree with one:

```text
sign, coefficient (digits), exponent10, and an optional NaN / Infinity
```

That is enough to render exactly what the reference renders, and enough for a
backend to load into whatever it uses. Putting a crate in the model instead
would make every consumer of a value depend on a choice only arithmetic cares
about.

## The mechanism this needs

Today the executor checks in preflight that one step's declared output
satisfies the next step's declared input, and nothing converts between them.
The reference converts. So the model needs one place that projects a value of
kind A into kind B, reproducing the dish's own conversion -- and preflight
should accept a pair when a projection exists rather than only when the kinds
already match.

That projection layer, not the extra variants, is the substance of this work.
