## MODIFIED Requirements

### Requirement: Agent Core Must Drive Store-Backed Turns

The system MUST load persisted transcript state into `agent-runtime`, stream
runtime events, and persist the resulting transcript back into the store.

When runtime ATIF emission is enabled, the core MUST also persist the completed
ATIF trajectory associated with that turn.

#### Scenario: Turn persists updated transcript

- **WHEN** a caller starts a turn for a stored session
- **THEN** `agent-core` loads the stored transcript into a session engine
- **AND** forwards runtime events for that turn
- **AND** persists the resulting transcript back into the store after the turn
  completes

#### Scenario: Turn persists completed ATIF trajectory

- **WHEN** a turn emits a completed ATIF trajectory through `agent-runtime`
- **THEN** `agent-core` SHALL upsert that trajectory for the session before the
  turn is fully released

#### Scenario: Cancellation stops an active turn

- **WHEN** a caller cancels an active turn
- **THEN** the core stops the per-turn runtime engine
- **AND** emits a cancellation event for that turn
- **AND** releases the session so future turns may start
