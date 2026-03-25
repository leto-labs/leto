# brain-stores Delta Spec

## ADDED Requirements

### Requirement: Store Semantics Must Stabilize Before Compatibility Work
The system SHALL treat stabilized store semantics as a hard prerequisite for
future OpenCode compatibility work.

The prerequisite store surface SHALL cover:

- project ownership and lookup behavior
- session lifecycle and listing behavior
- message persistence and retrieval behavior
- credential persistence and retrieval behavior
- trajectory persistence where remote session state depends on it

#### Scenario: Compatibility work is evaluated against incomplete store behavior
- **WHEN** project, session, message, credential, or trajectory semantics are
  still being actively reworked
- **THEN** the OpenCode compatibility implementation SHALL remain blocked
- **AND** store cleanup SHALL be treated as prerequisite work rather than as an
  implementation detail to defer

### Requirement: Lifecycle Events Must Be Ready For Remote Observation
The system SHALL treat the store lifecycle-event model as a prerequisite for
future compatibility work that depends on remote observation and session-aware
UI updates.

Those prerequisite event semantics SHALL be stable enough to support later
mapping into the compatibility surface's event delivery model.

#### Scenario: Store events are needed for compatibility planning
- **WHEN** contributors evaluate whether Brain can support the targeted
  compatibility consumer with minimal frontend changes
- **THEN** stable project, session, and related lifecycle events SHALL be
  treated as required groundwork
- **AND** unresolved event semantics SHALL block compatibility implementation

### Requirement: Provider And Config Compatibility Depends On Stable Credential Semantics
The system SHALL treat provider/config compatibility as dependent on stable
credential-store behavior rather than on frontend-only workarounds.

#### Scenario: Provider configuration is planned for compatibility
- **WHEN** contributors plan compatibility for provider and config flows
- **THEN** they SHALL rely on stable credential persistence and retrieval
  behavior in Brain
- **AND** they SHALL not treat frontend hacks as an acceptable substitute for
  unfinished store work
