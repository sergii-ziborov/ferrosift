# CyberChef 11.3.0 JSON interchange

FerroSift imports and exports JSON recipe arrays in the CyberChef 11.3.0
browser save format when every executable operation has an exact registered
`CyberChefV11_3` alias.

## Recipe format

The top-level value is an array. Each supported step contains:

- `op`: the exact CyberChef operation name;
- `args`: positional JSON arguments;
- `disabled`: an optional boolean, defaulting to `false`;
- `breakpoint`: an optional boolean, defaulting to `false`.

## Limits

- Maximum serialized input or output: 1 MiB.
- Maximum recipe length: 4096 steps.
- Maximum executable list/map argument depth: 120 containers.
- Executable integers: JavaScript safe-integer range,
  `-9007199254740991..=9007199254740991`.

## Import behavior

Operation names use exact, case-sensitive `CyberChefV11_3` aliases. No fuzzy
matching, normalization, or fallback runtime is used.

The import result preserves the complete semantic source JSON within the
limits. Unknown operations or fields, invalid flags, missing fields, extra
arguments, incompatible types, unsafe integers, and excessive argument depth
produce ordered findings with stable codes and source-step positions.

An executable FerroSift recipe is available only when every step maps without
an error finding. Source re-export preserves JSON values, fields, numeric
tokens, and step order. Whitespace and object-key order are not byte-for-byte
contracts.

## Export behavior

Export requires every operation to be registered with exactly one CyberChef
11.3 alias. Named arguments are emitted in their declared positional order.
Missing required arguments, undeclared arguments, ambiguous aliases, unsafe
integers, excessive nesting, and output beyond the size limit return explicit
errors.

`disabled` and `breakpoint` are emitted only when true.

## Format boundaries

This JSON format does not accept CyberChef URL/deep-link recipes or the
human-readable Chef format. Operations without an exact registered CyberChef
11.3 alias are preserved as source findings and are not executable.
