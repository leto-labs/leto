# brain-tools Delta Spec

## MODIFIED Requirements

### Requirement: Native Tools Preset
The `native_tools()` function SHALL remain unchanged. A new `all_tools(config)` function MAY be added that combines native tools with MCP tools discovered from the project config.

#### Scenario: All tools includes MCP
- **WHEN** `all_tools(config)` is called and the config defines MCP servers
- **THEN** it SHALL return native tools plus all tools bridged from connected MCP servers

#### Scenario: All tools without MCP config
- **WHEN** `all_tools(config)` is called and no MCP servers are configured
- **THEN** it SHALL return only native tools (equivalent to `native_tools()`)

## ADDED Requirements

### Requirement: McpTransport Trait
The system SHALL define an `McpTransport` trait that abstracts the communication layer between the MCP client and server. It SHALL support sending JSON-RPC requests and receiving responses.

#### Scenario: Stdio transport
- **WHEN** an MCP server is configured with `transport = "stdio"`
- **THEN** the system SHALL spawn the server process and communicate via stdin/stdout JSON-RPC

#### Scenario: SSE transport
- **WHEN** an MCP server is configured with `transport = "sse"`
- **THEN** the system SHALL connect to the server's HTTP SSE endpoint for receiving messages and POST for sending

### Requirement: McpClient
The system SHALL provide an `McpClient` struct that manages a connection to a single MCP server. It SHALL support:
- `connect(config) -> Result<McpClient>` — establish connection and perform initialization handshake
- `list_tools() -> Result<Vec<McpToolDef>>` — discover available tools
- `call_tool(name, args) -> Result<serde_json::Value>` — invoke a tool
- `disconnect()` — gracefully close the connection

#### Scenario: Connect and list tools
- **WHEN** `McpClient::connect(config)` is called with a valid server config
- **THEN** it SHALL establish a connection, perform the MCP `initialize` handshake
- **AND** `list_tools()` SHALL return the server's available tools

#### Scenario: Call tool
- **WHEN** `call_tool("read_file", args)` is called
- **THEN** it SHALL send a `tools/call` JSON-RPC request and return the result

#### Scenario: Server not available
- **WHEN** the MCP server is not running or reachable
- **THEN** `connect()` SHALL return an error

### Requirement: McpTool Bridge
The system SHALL provide an `McpTool` struct that implements the `Tool` trait by wrapping an MCP tool definition and delegating execution to an `McpClient`. MCP tools SHALL be indistinguishable from native tools to the agent loop.

#### Scenario: MCP tool as brain Tool
- **WHEN** an MCP server provides a tool named "read_file" with a JSON Schema
- **THEN** `McpTool` SHALL expose it with `definition()` returning a `ToolDef` with that name and schema
- **AND** `execute(args)` SHALL serialize args, call the MCP server, and return the result string

#### Scenario: MCP tool execution error
- **WHEN** the MCP server returns an error for a tool call
- **THEN** `execute()` SHALL return `Err` with the server's error message

### Requirement: Server Lifecycle Management
The system SHALL manage MCP server processes for stdio-based servers:
- Start the server process when tools are first needed
- Monitor the process for unexpected exits
- Restart the process on failure (with backoff)
- Gracefully shut down when the Brain is dropped or stopped

#### Scenario: Server auto-start
- **WHEN** an MCP tool is first invoked
- **THEN** the system SHALL ensure the MCP server process is running

#### Scenario: Server crash recovery
- **WHEN** an MCP server process exits unexpectedly
- **THEN** the system SHALL restart it with exponential backoff

#### Scenario: Graceful shutdown
- **WHEN** the Brain is dropped
- **THEN** all managed MCP server processes SHALL be gracefully terminated

### Requirement: MCP Tools Helper
The system SHALL provide an `mcp_tools(configs) -> Result<Vec<Arc<dyn Tool>>>` function that connects to multiple MCP servers and returns all their tools as `Arc<dyn Tool>` instances. This allows mixing MCP tools with native tools in `Brain::new()`.

#### Scenario: Multiple servers
- **WHEN** `mcp_tools(vec![server_a_config, server_b_config])` is called
- **THEN** it SHALL connect to both servers, list their tools, and return all tools as a flat Vec

