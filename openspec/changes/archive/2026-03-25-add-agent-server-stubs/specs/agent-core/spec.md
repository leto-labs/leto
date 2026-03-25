# agent-core Delta Spec

## ADDED Requirements

### Requirement: Agent Core Must Expose A Shared Consumer Boundary
The system MUST define an `AgentCore` trait in `agent-core` as the shared
consumer-facing boundary for embedded and future remote callers.

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
