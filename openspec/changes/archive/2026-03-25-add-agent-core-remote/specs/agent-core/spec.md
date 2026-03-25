# agent-core Delta Spec

## MODIFIED Requirements

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
