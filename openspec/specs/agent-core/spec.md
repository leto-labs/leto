# agent-core Specification

## Purpose
TBD - created by archiving change add-agent-core-and-store. Update Purpose after archive.
## Requirements
### Requirement: Agent Core Must Assemble The V2 Runtime Stack

The system MUST provide an `agent-core` crate that acts as the app-facing
composition boundary above `agent-runtime`.

#### Scenario: Default local assembly
- **WHEN** a caller builds the default local core surface
- **THEN** the core registers a default loop
- **AND** installs a native `agent-tools` tool executor
- **AND** requires at least one real registered or discovered provider

#### Scenario: Default local assembly rejects missing providers
- **WHEN** a caller builds the default local core surface with no registered providers and no discoverable credentials
- **THEN** the build fails with a no-providers error

#### Scenario: Credential-based provider discovery
- **WHEN** the backing store contains OpenAI-compatible API-key credentials
- **THEN** the core registers corresponding providers during default local assembly

### Requirement: Agent Core Must Drive Store-Backed Turns

The system MUST load persisted transcript state into `agent-runtime`, stream
runtime events, and persist the resulting transcript back into the store.

#### Scenario: Turn persists updated transcript
- **WHEN** a caller starts a turn for a stored session
- **THEN** `agent-core` loads the stored transcript into a session engine
- **AND** forwards runtime events for that turn
- **AND** persists the resulting transcript back into the store after the turn completes

#### Scenario: Cancellation stops an active turn
- **WHEN** a caller cancels an active turn
- **THEN** the core stops the per-turn runtime engine
- **AND** emits a cancellation event for that turn
- **AND** releases the session so future turns may start

### Requirement: Agent Core Must Expose A Shared Consumer Boundary
The system MUST define an `AgentCore` trait in `agent-core` as the shared
consumer-facing boundary for embedded and remote callers.

The shared boundary MUST cover:

- store access
- project and session orchestration
- transcript and trajectory access
- model helper methods
- turn execution and cancellation
- live event subscription

#### Scenario: Consumer depends on shared core only
- **WHEN** a caller such as a CLI, ACP adapter, web app, or hosted server
  depends on `Arc<dyn AgentCore>`
- **THEN** it SHALL not need to know whether the implementation is embedded or
  remote

#### Scenario: Remote implementation preserves shared boundary shape
- **WHEN** a caller uses `AgentCoreRemote`
- **THEN** the caller SHALL use the same `AgentCore` trait surface as
  `AgentCoreNative`
- **AND** locality SHALL remain hidden behind the shared boundary

### Requirement: Native Construction Must Stay Implementation-Specific
The system MUST provide `AgentCoreNative` as the first concrete implementation
of `AgentCore`.

`AgentCoreNativeBuilder` MAY take a concrete `Store` and local providers,
loops, and tools, but that construction pattern SHALL remain specific to the
native implementation rather than defining the shared boundary.

#### Scenario: Remote implementation does not inherit native builder constraints
- **WHEN** a future `AgentCoreRemote` is added
- **THEN** it SHALL be free to construct from transport/client configuration
- **AND** it SHALL not be required to accept a concrete local store

### Requirement: Shared Core Preserves Proxy Store Parity
The shared `AgentCore` boundary MUST continue to expose store access so future
remote implementations can return proxy store handles.

#### Scenario: Remote core returns proxy store handles
- **WHEN** a future `AgentCoreRemote` implements `AgentCore`
- **THEN** `store()` SHALL remain available
- **AND** callers SHALL be able to use the same store-oriented access pattern
  across native and remote implementations

