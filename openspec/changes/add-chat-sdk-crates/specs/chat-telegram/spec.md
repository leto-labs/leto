## ADDED Requirements

### Requirement: Chat-Telegram Provides A Shared Adapter Implementation

The system SHALL provide a `chat-telegram` crate that depends on the shared
`chat` crate and implements the shared `ChatAdapter` trait for Telegram.

#### Scenario: Telegram adapter exposes shared metadata and config

- **WHEN** a caller depends on `chat-telegram`
- **THEN** it SHALL be able to construct a Telegram adapter from
  Telegram-specific config
- **AND** consume the shared `chat::ChatAdapter` interface

### Requirement: Chat-Telegram Normalizes Telegram Updates

The Telegram adapter SHALL normalize Telegram message updates into the shared
chat event model.

#### Scenario: Telegram message update becomes a normalized shared event

- **WHEN** the Telegram adapter receives a Telegram message update
- **THEN** it SHALL emit a shared message-received or message-edited event
- **AND** preserve conversation ids, reply/thread metadata, sender identity,
  text, and attachments through shared chat types
