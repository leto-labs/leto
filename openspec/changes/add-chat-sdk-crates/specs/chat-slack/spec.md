## ADDED Requirements

### Requirement: Chat-Slack Provides A Shared Adapter Implementation

The system SHALL provide a `chat-slack` crate that depends on the shared
`chat` crate and implements the shared `ChatAdapter` trait for Slack.

#### Scenario: Slack adapter exposes shared metadata and config

- **WHEN** a caller depends on `chat-slack`
- **THEN** it SHALL be able to construct a Slack adapter from Slack-specific
  config
- **AND** consume the shared `chat::ChatAdapter` interface

### Requirement: Chat-Slack Normalizes Slack Events

The Slack adapter SHALL normalize Slack message events into the shared chat
event model.

#### Scenario: Slack threaded message becomes a normalized shared event

- **WHEN** the Slack adapter receives a Slack threaded message event
- **THEN** it SHALL emit a shared message-received or message-edited event
- **AND** preserve channel/thread metadata, sender identity, text, and files
  through shared chat types
