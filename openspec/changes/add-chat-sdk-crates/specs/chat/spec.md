## ADDED Requirements

### Requirement: Shared Chat SDK Crate

The system SHALL provide a standalone `chat` crate that owns the shared chat
adapter trait, normalized chat events, normalized message types, capability
metadata, and shared error model.

The crate SHALL NOT depend on `brain-types` or any other `brain-*` crate.

#### Scenario: Shared crate exposes reusable chat primitives

- **WHEN** a caller depends on `chat`
- **THEN** it SHALL be able to construct shared conversation, message,
  participant, and attachment types
- **AND** subscribe to normalized chat events without importing `brain-*`

### Requirement: Shared Chat Adapter Trait Uses Events And Async Operations

The shared `chat` crate SHALL define a `ChatAdapter` trait centered on:

- adapter metadata and capabilities
- normalized inbound event subscription
- outbound send operations
- optional edit and delete operations
- health checks

#### Scenario: Shared adapter exposes normalized event stream

- **WHEN** a concrete chat crate implements `ChatAdapter`
- **THEN** callers SHALL be able to subscribe to a stream of normalized chat
  events

#### Scenario: Unsupported optional operations fail explicitly

- **WHEN** a concrete adapter does not support message edits or deletes
- **THEN** the shared default implementation SHALL return an explicit
  unsupported-capability error

### Requirement: Shared Chat SDK Uses Capability Discovery

The shared `chat` crate SHALL model optional provider behavior through explicit
capability metadata rather than forcing every adapter to emulate the same
feature set.

The shared capability set SHALL be able to represent at least:

- inbound message receipt
- outbound message send
- threaded replies
- message edits
- typing indicators
- reactions
- attachments
- webhook mode
- polling mode
- health checks

#### Scenario: Concrete adapter publishes its supported feature set

- **WHEN** a caller inspects a concrete adapter
- **THEN** it SHALL be able to discover the supported optional capabilities
  through shared metadata
