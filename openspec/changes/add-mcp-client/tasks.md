# Tasks: add-mcp-client

## Implementation Checklist

- [ ] Define MCP JSON-RPC message types (Request, Response, Notification)
- [ ] Implement MCP stdio transport (spawn process, read/write JSON-RPC over stdin/stdout)
- [ ] Implement `tools/list` call → parse MCP tool definitions
- [ ] Implement `McpTool` struct that implements `Tool` trait (bridges MCP tool into brain)
- [ ] Implement `tools/call` execution → serialize args, send to MCP server, return result
- [ ] Implement `McpClient` struct: connect, list tools, call tools, disconnect
- [ ] Implement server lifecycle management for stdio servers (spawn and shutdown)
- [ ] Implement `mcp_tools(servers) -> Vec<Arc<dyn Tool>>` helper that connects to already-resolved server descriptors and returns bridged tools
- [ ] Handle MCP server errors gracefully (connection refused, timeout, invalid response)
- [ ] Write unit tests with mock MCP server (in-process)
- [ ] Write integration test with a real MCP server (e.g., echo server)
