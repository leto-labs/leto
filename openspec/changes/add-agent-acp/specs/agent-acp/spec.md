# agent-acp Specification Delta

## ADDED Requirements

### Requirement: Agent ACP Must Replace Legacy brain-acp In The Refactored Stack

The system MUST provide an `agent-acp` crate as the canonical ACP adapter for
the refactored `agent-*` stack.

`agent-acp` MUST replace `brain-acp` as the preferred ACP integration surface
for new work on the migrated architecture.

#### Scenario: New ACP integration targets the refactored crate
- **WHEN** a caller needs an ACP adapter for the refactored stack
- **THEN** it SHALL depend on `agent-acp`
- **AND** it SHALL treat `brain-acp` as the legacy ACP surface

### Requirement: Agent ACP Must Wrap AgentCore

The `agent-acp` backend MUST depend on the shared `AgentCore` boundary rather
than on legacy `BrainRuntime`.

The backend MUST:

- hold an `AgentCore` implementation in backend state
- use `AgentCore` project resolution and session orchestration methods
- use `AgentCore` turn execution and cancellation methods
- use `AgentCore` store-facing helpers for persisted session and message access

#### Scenario: ACP backend resolves sessions through AgentCore
- **WHEN** ACP creates, loads, prompts, or cancels a session
- **THEN** the backend SHALL perform that work through `AgentCore`
- **AND** it SHALL not require `BrainRuntime`

### Requirement: Agent ACP Must Preserve ACP Session Lifecycle Behavior

The `agent-acp` crate MUST support ACP session creation, loading, listing,
prompt execution, cancellation, and history replay on top of persisted
`agent-store` state.

#### Scenario: Loaded session replays persisted transcript state
- **WHEN** an ACP client loads an existing session through `agent-acp`
- **THEN** the backend SHALL replay the stored transcript as ACP session updates
- **AND** return the current ACP session config options for that session

#### Scenario: Prompt request streams AgentCore turn updates
- **WHEN** an ACP client prompts an `agent-acp` session
- **THEN** the backend SHALL stream mapped turn updates derived from `AgentCore`
- **AND** complete the ACP request with the mapped stop reason

### Requirement: Agent ACP Must Expose Session Config Through Agent Store State

The `agent-acp` crate MUST expose ACP session config options backed by
persisted `agent-store` session state.

The config surface MUST include:

- `model`
- `thought_level`
- `loop`

#### Scenario: Session config mutation persists through AgentCore
- **WHEN** an ACP client updates `model`, `thought_level`, or `loop`
- **THEN** `agent-acp` SHALL persist the change through `AgentCore`
- **AND** return the refreshed ACP config-option snapshot

### Requirement: Agent ACP Must Provide A Stdio ACP Entrypoint

The `agent-acp` crate MUST provide a stdio ACP entrypoint that serves the
`AgentCore`-backed ACP adapter.

#### Scenario: Stdio server runs the AgentCore-backed ACP adapter
- **WHEN** a caller launches the `agent-acp` stdio entrypoint
- **THEN** the process SHALL serve ACP over stdio using the `AgentCore`-backed
  adapter
