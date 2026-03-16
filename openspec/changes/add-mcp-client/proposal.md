# Proposal: add-mcp-client

## Why

The Model Context Protocol (MCP) has become the de facto standard for
connecting AI agents to external tool servers. OpenCode, Rig, Vercel AI SDK,
and most modern agent platforms support MCP. brain currently has zero MCP
integration — all tools must be compiled into the binary.

MCP support is critical for:

1. **Tool ecosystem access**: hundreds of MCP servers exist for databases,
   APIs, browsers, file systems, etc. Without MCP, brain can only use its
   built-in tools.
2. **User extensibility**: users can add tools to their agent without modifying
   brain's source code — just point to an MCP server in config.
3. **IDE integration**: both Cursor and VS Code use MCP for tool extensions.
   If brain will power IDE agents, it needs to speak MCP.
4. **Skills/commands**: the `.agents/skills/` convention maps naturally to
   MCP servers — each skill can expose tools via MCP.

## What

### MCP Client

A `brain-mcp` crate (or feature-gated module) that implements an MCP client:

1. **Transport support**:
   - `stdio` — spawn a child process, communicate via stdin/stdout JSON-RPC
   - `sse` — connect to an HTTP SSE endpoint (MCP's HTTP transport)
   - `streamable-http` — the newer MCP Streamable HTTP transport

2. **Tool discovery**: connect to an MCP server, call `tools/list`, and
   produce `Vec<Arc<dyn Tool>>` that bridge MCP tools into brain's `Tool` trait.

3. **Tool execution**: when a bridged MCP tool is called, serialize the
   arguments, send `tools/call` to the MCP server, and return the result.

4. **Resource/prompt support** (optional, lower priority): MCP also defines
   resources and prompts. These can be exposed as context sources for the
   agent loop.

5. **Server lifecycle**: manage MCP server processes (start, monitor, restart,
   shutdown). Stdio servers need process management; HTTP servers are external.

### Config integration

MCP servers declared in `.agents/config.toml`:

```toml
[[mcp.servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
transport = "stdio"

[[mcp.servers]]
name = "postgres"
url = "http://localhost:3001/mcp"
transport = "sse"
```

### Tool bridging

MCP tools appear as regular `Arc<dyn Tool>` instances to the rest of brain.
The agent loop doesn't need to know whether a tool is native or MCP-backed.

## Change Dependencies

- **Requires**: `add-config-system` (MCP servers declared in `[[mcp.servers]]` config section)
- **Requires**: `expand-tool-suite` (MCP tools bridge into the Tool trait; tool groups include MCP)

## Impact

- **New crate**: `brain-mcp` (feature-gated, default off)
- **Modifies**: `tool-system` (MCP tool bridge implements Tool trait)
- **New spec**: `mcp-client`
- **Dependencies**: `serde_json`, `tokio::process` (for stdio), `reqwest` +
  `eventsource-stream` (for SSE)
- **No breaking changes**: MCP tools are additive; existing tools unaffected
