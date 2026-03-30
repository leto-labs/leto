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

#### Scenario: Cancelled turn is not surfaced as ACP internal error

- **WHEN** the real ACP backend ends a turn because the session was cancelled
- **THEN** the prompt request SHALL complete with ACP's cancelled stop reason
- **AND** it SHALL NOT be surfaced as ACP `internal_error`

### Requirement: ACP Compatibility Launch Path Remains Available

The system SHALL keep `agent acp` as a supported local launch path for the
runtime-backed ACP stdio server.

The `agent-acp` crate SHALL continue to expose a direct stdio entrypoint for
repo-owned binaries and tests.

#### Scenario: Existing agent ACP command starts the backend

- **WHEN** a user launches `agent acp`
- **THEN** the ACP stdio server SHALL start successfully against the embedded
  `AgentCore`

#### Scenario: Repo-owned code can call the stdio entrypoint directly

- **WHEN** a repo-owned binary or test needs to start the real ACP backend
- **THEN** it SHALL be able to call the exported `agent_acp::run_stdio(...)`
  entrypoint
