# ARCHIVED SPEC

> [!NOTE] Historical archive snapshot. Use active canonical specs for current requirements.

## ADDED Requirements

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
The system SHALL include an `EchoTool` that logs its invocation via `tracing` and returns the input arguments as a formatted string. This tool exists for testing the tool dispatch path without side effects.

#### Scenario: Echo tool returns input
- **WHEN** EchoTool receives arguments `{"message": "test"}`
- **THEN** it SHALL log the invocation at info level
- **AND** it SHALL return the arguments formatted as a string

### Requirement: ToolDef Schema
Tool definitions SHALL provide a JSON Schema for their parameters via `serde_json::Value`. Implementations construct the schema using `serde_json::json!()` or any method that produces valid JSON Schema.

#### Scenario: Schema describes tool parameters
- **WHEN** a tool's `definition()` is called
- **THEN** `ToolDef::parameters` SHALL contain a valid JSON Schema describing accepted arguments
