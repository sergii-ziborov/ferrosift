# FerroSift

FerroSift is a pure-Rust runtime for deterministic, local-first data
transformation recipes. Its portable model and execution engine run on native
targets and `wasm32-unknown-unknown`.

## Capabilities

- Representation-preserving values for bytes, encoded text, booleans,
  integers, lists, maps, and virtual files.
- Versioned recipes with stable operation and step identifiers, typed named
  arguments, metadata, disabled steps, and breakpoints.
- Validated operation contracts covering input/output values, defaults,
  execution targets, capabilities, aliases, and evidence records.
- Deterministic operation registration and exact profile-scoped alias lookup.
- Forty-one built-in pure-Rust operations spanning encoding, hashing (MD5,
  SHA-1/2, HMAC), compression (Gzip/Gunzip, Zlib), logic (XOR), data
  slicing, HTML entities, ROT13, charcodes, and Find / Replace, each pinned
  to its CyberChef 11.3.0 counterpart where aliases exist.
- Complete preflight before execution, preventing partial side effects when a
  later step is invalid.
- Explicit input, output, and expansion budgets.
- Cooperative cancellation and preflight verification of declared capability
  grants; the portable core exposes no built-in host or network handles.
- Bounded execution traces containing locations and value summaries rather
  than recipe payloads.
- Loss-aware JSON interchange for
  [CyberChef 11.3.0 recipes](docs/compatibility/cyberchef-v11.3.0.md) whose
  operations have exact registered aliases.
- Native `ferrosift` CLI with `operations`, `describe`, `validate`, and `run`
  commands for bounded file or standard-stream processing.

## Compatibility and attribution

FerroSift is independent of and not endorsed by GCHQ. CyberChef is a separate
project distributed under the Apache License 2.0 and Crown Copyright.

## License

Apache-2.0.
