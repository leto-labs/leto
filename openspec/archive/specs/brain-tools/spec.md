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

The native `file_read` implementation SHALL page output safely for model use.

#### Scenario: Read entire file
- **WHEN** `file_read` is called with `{"path": "foo.txt"}`
- **THEN** it SHALL return a bounded default slice of lines rather than the full
  file
- **AND** if more content remains it SHALL include a continuation hint using
  `offset`

#### Scenario: Read with offset and limit
- **WHEN** `file_read` is called with `{"path": "foo.txt", "offset": 5, "limit": 10}`
- **THEN** it SHALL return lines 5 through 14 with line numbers
- **AND** if more content remains it SHALL include a continuation hint using
  `offset`

#### Scenario: Read truncates long lines
- **WHEN** a returned line exceeds the native line-length cap
- **THEN** the native driver SHALL truncate the displayed line content

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

The native `shell` implementation SHALL bound returned command output.

#### Scenario: Shell executes command
- **WHEN** `shell` is called with `{"command": "echo hello"}`
- **THEN** it SHALL execute the command and return bounded stdout/stderr

#### Scenario: Shell output exceeds native cap
- **WHEN** combined stdout/stderr exceeds the native output budget
- **THEN** the tool SHALL return a truncated output preview
- **AND** it SHALL include a truncation notice

### Requirement: Native Glob Search

The native `glob_search` implementation SHALL cap large result sets.

#### Scenario: Glob finds matching files
- **WHEN** `glob_search` is called with `{"pattern": "**/*.rs"}`
- **THEN** it SHALL return a bounded list of matching file paths

#### Scenario: Glob result set exceeds native cap
- **WHEN** more than the native maximum number of matches are found
- **THEN** the tool SHALL return only the bounded prefix of matches
- **AND** it SHALL include a truncation notice

### Requirement: Native Grep

The native `grep` implementation SHALL cap large result sets and long match
lines.

#### Scenario: Grep finds matches
- **WHEN** `grep` is called with `{"pattern": "fn main"}`
- **THEN** it SHALL return bounded matching lines with file path and line number

#### Scenario: Grep result set exceeds native cap
- **WHEN** `grep` finds more than the native maximum number of matches
- **THEN** it SHALL return only the bounded prefix of matches
- **AND** it SHALL include a truncation notice

#### Scenario: Grep match line exceeds native cap
- **WHEN** a matching line exceeds the native line-length cap
- **THEN** the tool SHALL truncate the displayed line content

### Requirement: Native Tools Preset

The native tool preset MUST include an `apply_patch` tool for structured file
changes.

#### Scenario: Native preset includes apply_patch
- **WHEN** native tools are constructed
- **THEN** the preset includes `apply_patch`

### Requirement: ApplyPatch Tool

The `apply_patch` tool MUST accept patch text in the V4A patch-envelope format
used by Codex-family agents.

#### Scenario: Add file patch succeeds
- **WHEN** the caller submits a patch containing `*** Begin Patch`,
  `*** Add File:`, `+`-prefixed file contents, and `*** End Patch`
- **THEN** the tool creates the file

#### Scenario: Update patch succeeds
- **WHEN** the caller submits a patch containing `*** Update File:` and one or
  more `@@` hunks with context and `+` / `-` lines
- **THEN** the tool updates the file contents

#### Scenario: Delete patch succeeds
- **WHEN** the caller submits a patch containing `*** Delete File:`
- **THEN** the tool deletes the file

#### Scenario: Move patch succeeds
- **WHEN** the caller submits a patch containing `*** Update File:` followed by
  `*** Move to:`
- **THEN** the tool writes the updated contents at the new relative path and
  removes the original file

The `apply_patch` tool MUST reject malformed V4A input with explicit tool
errors.

#### Scenario: Missing envelope markers
- **WHEN** the patch omits `*** Begin Patch` or `*** End Patch`
- **THEN** the tool returns an error describing the missing markers

#### Scenario: Empty patch
- **WHEN** the patch contains no file operations
- **THEN** the tool returns an error indicating the patch is empty

#### Scenario: Absolute path
- **WHEN** a file operation references an absolute path
- **THEN** the tool returns an error indicating only relative paths are allowed

### Requirement: ListDirectory Tool

The native `list_directory` implementation SHALL cap recursive listings.

#### Scenario: Directory listing returns tree output
- **WHEN** `list_directory` is called on a nested directory
- **THEN** it SHALL return a readable tree-like listing
- **AND** omit ignored generated directories

#### Scenario: Directory walk exceeds native entry cap
- **WHEN** a recursive listing discovers more than the native maximum number of
  entries
- **THEN** the tool SHALL stop after the bounded prefix
- **AND** it SHALL include a truncation notice

### Requirement: Grep Prefers Ripgrep When Available

The `grep` tool SHALL keep its public name and schema while allowing a
ripgrep-backed implementation internally.

The native preset SHALL:

- use a ripgrep-backed driver when `rg` is available
- fall back to the existing native grep implementation otherwise

#### Scenario: Native tool preset keeps grep contract stable

- **WHEN** `native_tools()` is constructed
- **THEN** it SHALL still expose a tool named `grep`
- **AND** the choice of native or ripgrep-backed implementation SHALL remain an
  internal detail

### Requirement: Native Terminal Session Tool

The native tool registry SHALL provide a stateful terminal-session tool for
 persistent shell interaction.

The tool SHALL support:

- session-scoped persistent shell state
- verbatim keystroke injection
- bounded waiting before output capture
- timeout reporting with current terminal state
- incremental output retrieval across loop iterations

#### Scenario: Session-scoped command preserves shell state

- **WHEN** the tool is called multiple times for the same session
- **THEN** later calls SHALL observe filesystem and shell state created by
  earlier calls

#### Scenario: Timed wait returns current terminal state

- **WHEN** the tool is invoked with keystrokes and a bounded wait interval
- **THEN** it SHALL return the current terminal state after the wait interval
- **AND** if the command exceeds the wait budget it SHALL return a timeout-style
  observation rather than tearing down the session

### Requirement: Native Tools Preset Includes Terminal Session Tool

The native tool preset SHALL include the terminal-session tool in addition to
 the existing native tools.

#### Scenario: Native preset registers terminal session tool

- **WHEN** `native_tools()` is constructed
- **THEN** the preset SHALL include the terminal-session tool

