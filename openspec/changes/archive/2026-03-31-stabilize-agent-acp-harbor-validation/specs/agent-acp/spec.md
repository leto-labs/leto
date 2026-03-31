## MODIFIED Requirements

### Requirement: ACP Stdio Keeps Protocol Output Separate From Logs

The ACP stdio launch path SHALL keep protocol output isolated from
human-readable tracing and backend logs.

The real ACP backend SHALL also treat user-driven cancellation as a normal ACP
outcome rather than as an internal protocol failure.

Repo-owned ACP validation clients SHALL be able to capture prompt-failure
debug information without polluting ACP protocol stdout.

#### Scenario: Cancelled turn is not surfaced as ACP internal error

- **WHEN** the real ACP backend ends a turn because the session was cancelled
- **THEN** the prompt request SHALL complete with ACP's cancelled stop reason
- **AND** it SHALL NOT be surfaced as ACP `internal_error`

#### Scenario: Prompt failure diagnostics do not corrupt ACP stdout

- **WHEN** a repo-owned ACP client captures backend logs and a prompt fails
- **THEN** human-readable diagnostics SHALL remain in stderr or client-owned
  debug artifacts
- **AND** ACP protocol stdout SHALL remain valid for the client transport

## ADDED Requirements

### Requirement: ACP Tool Lifecycles Use One Pending Call Followed By Updates

The real `agent-acp` backend SHALL expose one coherent ACP tool lifecycle for
each runtime tool invocation.

The backend SHALL:

- emit one pending `ToolCall` when the runtime first surfaces the call
- emit `ToolCallUpdate` messages for in-progress and completed transitions
- avoid replaying duplicate pending tool calls from committed assistant
  transcript messages after runtime tool events already emitted them

#### Scenario: Runtime tool call advances through ACP updates

- **WHEN** the runtime surfaces a pending, started, and completed tool call
- **THEN** ACP SHALL emit one pending `ToolCall`
- **AND** subsequent lifecycle changes SHALL be emitted as `ToolCallUpdate`

#### Scenario: Transcript replay does not duplicate a runtime-owned tool call

- **WHEN** the runtime already emitted ACP updates for a tool call
- **AND** the committed assistant transcript later includes the same tool call
- **THEN** the ACP adapter SHALL not emit a second pending `ToolCall` for that
  same runtime-owned invocation
