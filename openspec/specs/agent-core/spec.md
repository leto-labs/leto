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

