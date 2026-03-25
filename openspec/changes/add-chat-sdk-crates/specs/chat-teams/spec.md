## ADDED Requirements

### Requirement: Chat-Teams Provides A Shared Adapter Implementation

The system SHALL provide a `chat-teams` crate that depends on the shared `chat`
crate and implements the shared `ChatAdapter` trait for Microsoft Teams.

#### Scenario: Teams adapter exposes shared metadata and config

- **WHEN** a caller depends on `chat-teams`
- **THEN** it SHALL be able to construct a Teams adapter from Teams-specific
  config
- **AND** consume the shared `chat::ChatAdapter` interface

### Requirement: Chat-Teams Normalizes Teams Activities

The Teams adapter SHALL normalize Teams message activities into the shared chat
event model.

#### Scenario: Teams activity becomes a normalized shared event

- **WHEN** the Teams adapter receives a Teams message activity
- **THEN** it SHALL emit a shared message-received or message-edited event
- **AND** preserve conversation identifiers, sender identity, reply metadata,
  text, and attachments through shared chat types
