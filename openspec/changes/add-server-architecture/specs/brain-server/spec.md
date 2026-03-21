# brain-server Delta Spec

## ADDED Requirements

### Requirement: Remote Runtime Parity
The future server architecture SHALL support a remote runtime implementation
that preserves the same client-facing `BrainRuntime` concepts used by embedded
clients.

That remote runtime SHALL support:

- project resolution by root
- turn execution and cancellation
- runtime-bus subscription
- model listing and model selection
- store-backed project, session, message, and credential access
- provider, tool, and loop registry access

#### Scenario: Embedded and remote clients share one boundary
- **WHEN** a caller uses either `BrainRuntimeNative` or `BrainRuntimeRemote`
- **THEN** it SHALL work through the same `BrainRuntime` trait

### Requirement: Remote Store And Registry Proxies
The future remote runtime SHALL provide proxy implementations of the runtime's
store and registry handles so client code does not need a second API shape for
remote operation.

#### Scenario: Remote runtime exposes proxy store handles
- **WHEN** a caller uses `runtime.store()` on `BrainRuntimeRemote`
- **THEN** it SHALL receive proxy store objects implementing the same store traits

#### Scenario: Remote runtime exposes proxy registries
- **WHEN** a caller uses `runtime.providers()`, `runtime.tools()`, or `runtime.loops()` on `BrainRuntimeRemote`
- **THEN** it SHALL receive proxy registry objects implementing the same registry traits

### Requirement: Remote Turn And Runtime-Bus Delivery
The future remote server transport SHALL preserve both per-turn streaming and
live runtime-bus subscription.

#### Scenario: Remote turn preserves event streaming
- **WHEN** a caller starts a turn through `BrainRuntimeRemote`
- **THEN** it SHALL receive the streamed turn events for that turn

#### Scenario: Remote runtime preserves live bus subscription
- **WHEN** a caller subscribes to the runtime bus through `BrainRuntimeRemote`
- **THEN** it SHALL receive the live runtime bus events emitted by the hosted runtime

### Requirement: Hosted Native Runtime
The future server process SHALL host `BrainRuntimeNative` as its core runtime
rather than introducing a separate primary client boundary centered on
`BrainApi`.

#### Scenario: Server hosts native runtime
- **WHEN** `brain-server` is redesigned for the remote-runtime path
- **THEN** it SHALL host `BrainRuntimeNative`
- **AND** expose remote transport around that runtime rather than around a distinct client API

### Requirement: Legacy BrainApi Is Not The Target Architecture
The future remote/server architecture SHALL target `BrainRuntime` parity
through `BrainRuntimeRemote` rather than `BrainApi`. Existing `BrainApi` and
`BrainServer` code may remain as historical implementation material while the
server crate is out of the active workspace, but they SHALL NOT define the
target client boundary for the future design.

#### Scenario: Legacy server code does not define the future boundary
- **WHEN** the remote-runtime architecture is documented or implemented
- **THEN** `BrainRuntime` and `BrainRuntimeRemote` SHALL be treated as the target boundary
- **AND** `BrainApi` SHALL be treated as legacy code to replace or retire

### Requirement: Server Work Is Deferred
The remote-runtime/server path SHALL NOT be the active client boundary today.

#### Scenario: Current clients are not blocked on server work
- **WHEN** evaluating the current embedded CLI and ACP backends
- **THEN** they SHALL be able to operate directly on `BrainRuntimeNative`
- **AND** SHALL not require `brain-server` to be present in the active workspace
