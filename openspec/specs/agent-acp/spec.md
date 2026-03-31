# agent-acp Specification

## Purpose
ACP stdio adapter over the current agent stack. Defines the runtime-backed
backend, ACP-facing session configuration, and the supported local launch
surfaces that exist today.
## Requirements
### Requirement: ACP Backend Depends On AgentCore
The real ACP backend SHALL depend on the shared `AgentCore` boundary rather
than on legacy `Brain`-era composition.

The backend SHALL:

- hold a core trait object in app state
- use `core.resolve_or_create_project()` for cwd-to-project bootstrap
- use `core.turn()` and turn-cancellation methods for active turn lifecycle
- use `AgentCore` session/runtime helper methods for effective model, loop, and
  runtime-config inspection
- use `core.store()` for plain session/message/project CRUD

#### Scenario: Real backend uses core for project and turn handling
- **WHEN** ACP creates or prompts a real session
- **THEN** it SHALL resolve the project through `AgentCore`
- **AND** execute and cancel turns through `AgentCore`

#### Scenario: Real backend loads session state through the core
- **WHEN** ACP loads an existing session
- **THEN** it SHALL resolve the session and replay session history through
  `AgentCore` rather than ACP-local shadow state

### Requirement: Real Backend Exposes ACP Config Options

The real `agent-acp` backend SHALL expose ACP `configOptions` for session
settings.

The real surface SHALL include:

- `model` when the session resolves an effective model
- `thought_level`
- `loop` when one or more loops are registered

The backend SHALL include those config options on both `session/new` and
`session/load`.

#### Scenario: Real backend omits loop config when no loops are registered
- **WHEN** ACP creates or loads a real backend session with no registered loops
- **THEN** the response SHALL omit the `loop` config option

#### Scenario: Real backend exposes loop config when loops are registered
- **WHEN** ACP creates or loads a real backend session with one or more
  registered loops
- **THEN** the response SHALL include a `loop` config option

### Requirement: Real Backend Supports Session Config Mutation

The real `agent-acp` backend SHALL implement `session/set_config_option`.

The backend SHALL:

- accept `model` updates
- accept `thought_level` updates
- accept `loop` updates
- return the full current config-option snapshot after each update

#### Scenario: Loop update returns full config options
- **WHEN** ACP sets the real backend session `loop`
- **THEN** the backend SHALL persist the session loop override
- **AND** return the full current `configOptions`

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

### Requirement: ACP Compatibility Launch Path Remains Available

The system SHALL keep `agent acp` as a supported local launch path for the
runtime-backed ACP stdio server.

The `agent-acp` crate SHALL continue to expose a direct stdio entrypoint for
repo-owned binaries and tests.

The repo SHALL also expose:

- a direct `agent-acp` binary identity for repo-owned ACP integrations
- an explicit `agent-acp-mock` launch path for ACP smoke validation

#### Scenario: Existing agent ACP command starts the backend

- **WHEN** a user launches `agent acp`
- **THEN** the ACP stdio server SHALL start successfully against the embedded
  `AgentCore`

#### Scenario: Repo-owned code can call the stdio entrypoint directly

- **WHEN** a repo-owned binary or test needs to start the real ACP backend
- **THEN** it SHALL be able to call the exported `agent_acp::run_stdio(...)`
  entrypoint

#### Scenario: Direct agent-acp binary is available

- **WHEN** a repo-owned integration such as Harbor needs the real ACP backend
- **THEN** it SHALL be able to launch a direct `agent-acp` binary identity

#### Scenario: ACP smoke validation keeps an explicit mock lane

- **WHEN** the repo runs the `acpx` compatibility harness
- **THEN** it SHALL be able to launch an explicit `agent-acp-mock` backend
  without requiring external model credentials

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

