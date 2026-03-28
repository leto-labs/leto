# agent-cli Specification Delta

## ADDED Requirements

### Requirement: Agent CLI Must Replace Legacy brain-cli For The Refactored Stack

The system MUST provide an `agent-cli` binary crate as the canonical local CLI
for the refactored `agent-*` stack.

`agent-cli` MUST replace `brain-cli` as the preferred local interactive and
administrative surface for new work on the migrated architecture.

#### Scenario: Local user launches the refactored CLI
- **WHEN** a caller needs the local interactive/admin CLI on the refactored stack
- **THEN** it SHALL use `agent-cli`
- **AND** it SHALL treat `brain-cli` as the legacy local CLI

### Requirement: Agent CLI Must Embed AgentCore

The `agent-cli` crate MUST embed an in-process `AgentCore` implementation
rather than the legacy `BrainRuntime` surface.

The CLI MUST:

- open a file-backed `agent-store`
- build an embedded `AgentCore` implementation
- resolve the current project through `AgentCore`
- create and resume sessions through `AgentCore`
- execute turns through `AgentCore`

#### Scenario: Interactive chat runs through AgentCore
- **WHEN** the local CLI runs without a subcommand
- **THEN** it SHALL create or load a persisted session through `AgentCore`
- **AND** execute each prompt through `AgentCore`

### Requirement: Agent CLI Must Preserve Credential And Session Administration

The `agent-cli` crate MUST provide credential and session administration for
the refactored stack.

The supported surface MUST include:

- `credentials add`
- `credentials login`
- `credentials list`
- `credentials remove`
- `sessions list`
- `sessions resume`

#### Scenario: Credential login stores OAuth credentials
- **WHEN** a caller runs `agent credentials login openai-oauth`
- **THEN** the CLI SHALL complete the OAuth flow
- **AND** persist the resulting credential in `agent-store`

#### Scenario: Session resume continues an existing session
- **WHEN** a caller runs `agent sessions resume <ULID>`
- **THEN** the CLI SHALL load the stored session through `AgentCore`
- **AND** continue the conversation on that session
