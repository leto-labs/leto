# Proposal: add-mcp-client

## Why

The Model Context Protocol (MCP) has become the de facto standard for
connecting AI agents to external tool servers. OpenCode, Rig, Vercel AI SDK,
and most modern agent platforms support MCP. brain currently has zero MCP
integration, so all tools must be compiled into the binary.

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

### MCP Client MVP

A `brain-mcp` crate or feature-gated module that implements an MCP client for
stdio servers:

1. **Transport support**: spawn a child process and communicate via
   stdin/stdout JSON-RPC.
2. **Tool discovery**: connect to an MCP server, call `tools/list`, and
   produce bridged tools that implement brain's `Tool` trait.
3. **Tool execution**: when a bridged MCP tool is called, serialize the
   arguments, send `tools/call` to the MCP server, and return the result.
4. **Server lifecycle**: manage stdio server processes, including spawn and
   shutdown for the tool bridge.
5. **Bootstrap integration**: accept already-resolved MCP server descriptors
   from application bootstrap code. Config parsing is a separate concern and is
   not required for the MVP.

### Tool bridging

MCP tools appear as regular `Arc<dyn Tool>` instances to the rest of brain.
The agent loop doesn't need to know whether a tool is native or MCP-backed.

## Change Dependencies

- **Coordinates with**: the current `Tool` trait and `brain-tools` surface because MCP tools bridge into that existing abstraction
- **Coordinates with**: application bootstrap code that resolves which MCP servers to connect to

## Impact

- **New crate**: `brain-mcp` (feature-gated, default off)
- **Modifies**: `brain-tools` surface only if a helper is added for bridged MCP tool sets
- **New spec**: `mcp-client`
- **Dependencies**: `serde_json`, `tokio::process`
- **No breaking changes**: MCP tools are additive; existing tools unaffected
