# brain-tools Delta Spec

## MODIFIED Requirements

### Requirement: Native Tools Preset
The `native_tools()` function SHALL remain unchanged. A new `mcp_tools(servers)` helper MAY be added that converts already-resolved MCP server descriptors into bridged MCP tools.

#### Scenario: MCP tools helper uses resolved servers
- **WHEN** `mcp_tools(servers)` is called with one or more resolved MCP server descriptors
- **THEN** it SHALL return tools bridged from those servers

#### Scenario: Native tools remain unchanged
- **WHEN** `native_tools()` is called
- **THEN** it SHALL return only the native-backed tools

## ADDED Requirements

### Requirement: MCP Stdio Transport
The system SHALL define a stdio MCP transport for spawning a server process and exchanging JSON-RPC messages over stdin/stdout.

#### Scenario: Stdio transport starts a process
- **WHEN** an MCP client is created for a stdio server descriptor
- **THEN** the transport SHALL spawn the configured process and connect to its stdin/stdout streams

#### Scenario: Stdio transport is bidirectional
- **WHEN** the transport sends a JSON-RPC request
- **THEN** it SHALL read the corresponding response from the same stdio channel

### Requirement: McpClient
The system SHALL provide an `McpClient` struct that manages a connection to a single stdio MCP server. It SHALL support:
- `connect(server) -> Result<McpClient>` — establish connection and perform initialization handshake
- `list_tools() -> Result<Vec<McpToolDef>>` — discover available tools
- `call_tool(name, args) -> Result<serde_json::Value>` — invoke a tool
- `disconnect()` — gracefully close the connection

#### Scenario: Connect and list tools
- **WHEN** `McpClient::connect(server)` is called with a valid resolved stdio server descriptor
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
- Gracefully shut down when the Brain is dropped or stopped

#### Scenario: Server auto-start
- **WHEN** an MCP tool is first invoked
- **THEN** the system SHALL ensure the MCP server process is running

#### Scenario: Graceful shutdown
- **WHEN** the Brain is dropped
- **THEN** all managed MCP server processes SHALL be gracefully terminated
