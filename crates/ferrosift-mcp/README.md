# ferrosift-mcp

Local stdio MCP adapter over `ferrosift-host`. The portable library stays free of
Tokio and MCP protocol types; this binary is the transport.

```bash
cargo run -p ferrosift-mcp -- --root /path/to/samples
```

Logs go to stderr. stdout is reserved for the MCP JSON-RPC stream.

## Licence

Apache-2.0.
