# ferrosift-host

Shared hosted layer for FerroSift adapters: allowlisted input, opaque artifact
handles, catalog search, and recipe execution that returns a typed report.

The portable library stays free of filesystem and network handles. This crate is
the boundary that opens files, retains intermediate values, and enforces store
quotas before CLI or MCP surfaces them.

## Licence

Apache-2.0.
