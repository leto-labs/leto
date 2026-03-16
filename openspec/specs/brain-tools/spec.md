# brain-tools Specification

## Purpose
Tool interface and implementations. Defines the `Tool` trait, `ToolDef` schema, driver trait pattern for platform-specific adapters, and native tool implementations (file read/write/edit, shell, glob, grep).
## Requirements
### Requirement: Tool Trait
The system SHALL define a `Tool` trait with two methods: `definition()` returning a `ToolDef` (name, description, JSON Schema parameters), and `execute()` accepting `serde_json::Value` arguments and returning a `Result<String>`.

#### Scenario: Tool provides its schema
- **WHEN** `definition()` is called on a Tool
- **THEN** it SHALL return a `ToolDef` with name, description, and a JSON Schema for its parameters

#### Scenario: Tool executes with arguments
- **WHEN** `execute()` is called with valid JSON arguments
- **THEN** it SHALL return Ok with a string result

#### Scenario: Tool rejects invalid arguments
- **WHEN** `execute()` is called with arguments that don't match the schema
- **THEN** it SHALL return Err with a descriptive error

### Requirement: EchoTool
The system SHALL include an `EchoTool` in the `brain-tools` crate that logs its invocation via `tracing` and returns the input message as a string. This tool implements `Tool` directly with no driver trait.

#### Scenario: Echo tool returns input
- **WHEN** EchoTool receives arguments `{"message": "test"}`
- **THEN** it SHALL log the invocation at info level
- **AND** it SHALL return the message string

### Requirement: ToolDef Schema
Tool definitions SHALL provide a JSON Schema for their parameters via `serde_json::Value`. Implementations construct the schema using `serde_json::json!()` or any method that produces valid JSON Schema.

#### Scenario: Schema describes tool parameters
- **WHEN** a tool's `definition()` is called
- **THEN** `ToolDef::parameters` SHALL contain a valid JSON Schema describing accepted arguments

### Requirement: Driver Trait Pattern
Each tool capability SHALL be defined by a driver trait with typed parameters (no JSON). A generic tool adapter struct SHALL implement the `Tool` trait by parsing JSON arguments and delegating to the driver. Platform-specific drivers SHALL implement the driver trait using OS-specific APIs.

#### Scenario: Driver trait defines typed operation
- **WHEN** a driver trait is defined for a tool capability
- **THEN** its methods SHALL accept typed Rust parameters (not `serde_json::Value`)
- **AND** the trait SHALL require `Send + Sync`

#### Scenario: Tool adapter wraps driver into Tool
- **WHEN** a tool adapter is constructed with a driver
- **THEN** calling `definition()` SHALL return the JSON schema for the tool
- **AND** calling `execute()` SHALL parse JSON arguments and delegate to the driver

### Requirement: Native File Read
The system SHALL provide `FileReadDriver` trait and `FileReadDriverNative` implementation. `FileReadTool<T>` SHALL expose tool name `file_read` with parameters `path` (required string), `offset` (optional integer, 1-indexed line), `limit` (optional integer, max lines). The native driver SHALL read files using `tokio::fs` and format output with line numbers.

#### Scenario: Read entire file
- **WHEN** `file_read` is called with `{"path": "foo.txt"}`
- **THEN** it SHALL return the full file contents with line numbers

#### Scenario: Read with offset and limit
- **WHEN** `file_read` is called with `{"path": "foo.txt", "offset": 5, "limit": 10}`
- **THEN** it SHALL return lines 5 through 14 with line numbers

### Requirement: Native File Write
The system SHALL provide `FileWriteDriver` trait and `FileWriteDriverNative` implementation. `FileWriteTool<T>` SHALL expose tool name `file_write` with parameters `path` (required string) and `content` (required string). The native driver SHALL create parent directories and write using `tokio::fs`.

#### Scenario: Write creates file
- **WHEN** `file_write` is called with path and content
- **THEN** it SHALL create or overwrite the file with the given content
- **AND** it SHALL return a confirmation message

### Requirement: Native File Edit
The system SHALL provide `FileEditDriver` trait and `FileEditDriverNative` implementation. `FileEditTool<T>` SHALL expose tool name `file_edit` with parameters `path` (required string), `old_string` (required string), and `new_string` (required string). The native driver SHALL replace exactly one occurrence of `old_string` with `new_string`, failing if zero or multiple matches exist.

#### Scenario: Edit replaces unique match
- **WHEN** `file_edit` is called and `old_string` appears exactly once
- **THEN** it SHALL replace it with `new_string`
- **AND** it SHALL return a confirmation message

#### Scenario: Edit rejects ambiguous match
- **WHEN** `file_edit` is called and `old_string` appears more than once
- **THEN** it SHALL return an error indicating the match is not unique

### Requirement: Native Shell
The system SHALL provide `ShellDriver` trait and `ShellDriverNative` implementation. `ShellTool<T>` SHALL expose tool name `shell` with parameters `command` (required string) and `working_directory` (optional string). The native driver SHALL execute commands via `tokio::process::Command` and return combined stdout and stderr.

#### Scenario: Shell executes command
- **WHEN** `shell` is called with `{"command": "echo hello"}`
- **THEN** it SHALL execute the command and return stdout/stderr

### Requirement: Native Glob Search
The system SHALL provide `GlobDriver` trait and `GlobDriverNative` implementation. `GlobTool<T>` SHALL expose tool name `glob_search` with parameters `pattern` (required string) and `path` (optional string, base directory). The native driver SHALL find files matching the glob pattern and return the list of paths.

#### Scenario: Glob finds matching files
- **WHEN** `glob_search` is called with `{"pattern": "**/*.rs"}`
- **THEN** it SHALL return a newline-separated list of matching file paths

### Requirement: Native Grep
The system SHALL provide `GrepDriver` trait and `GrepDriverNative` implementation. `GrepTool<T>` SHALL expose tool name `grep` with parameters `pattern` (required string), `path` (optional string, base directory), and `include` (optional string, file glob filter). The native driver SHALL search file contents by regex and return matching lines with file paths, line numbers, and context.

#### Scenario: Grep finds matches
- **WHEN** `grep` is called with `{"pattern": "fn main"}`
- **THEN** it SHALL return matching lines with file path and line number

### Requirement: Native Tools Preset
The system SHALL provide a `native_tools()` function (feature-gated under `native`) that returns `Vec<Arc<dyn Tool>>` containing all native-driver-backed tools plus EchoTool.

#### Scenario: Preset returns all native tools
- **WHEN** `native_tools()` is called
- **THEN** it SHALL return a vector containing EchoTool and all 6 native tool adapters


