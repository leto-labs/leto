## ADDED Requirements

### Requirement: TerminusKiraLoop Provides Native Tool-Calling Terminal Control

The system SHALL provide a `TerminusKiraLoop` as an additive `AgentLoop`
implementation.

`TerminusKiraLoop` SHALL:

- call the provider with native semantic tool definitions rather than a
  text-only JSON protocol
- expose KIRA-style semantic tools named `execute_commands`, `task_complete`,
  and `image_read`
- execute command batches against a persistent terminal session
- support marker-based polling so command batches may complete before the full
  requested wait duration

#### Scenario: Execute commands tool call drives terminal batch

- **WHEN** the provider returns an `execute_commands` tool call
- **THEN** `TerminusKiraLoop` SHALL emit the assistant message and tool-call
  events
- **AND** execute the requested command batch through the terminal-session
  primitive
- **AND** continue the loop using the resulting observation

### Requirement: TerminusKiraLoop Supports Multimodal Image Read

`TerminusKiraLoop` SHALL support a KIRA-style `image_read` action by issuing a
multimodal provider subcall with text plus image content.

#### Scenario: Image read analyzes png file

- **WHEN** the provider returns an `image_read` request for a PNG file with an
  analysis instruction
- **THEN** the loop SHALL read and encode the image
- **AND** perform a multimodal provider call
- **AND** feed the resulting analysis back into the main loop as the next
  observation

### Requirement: TerminusKiraLoop Requires Double Completion Confirmation

`TerminusKiraLoop` SHALL require a second explicit completion confirmation
before ending the turn.

#### Scenario: First completion request returns checklist

- **WHEN** the provider requests `task_complete` while completion is not yet
  pending
- **THEN** the loop SHALL return a completion-confirmation checklist and
  continue running

#### Scenario: Second completion request ends turn

- **WHEN** the provider requests `task_complete` while completion is already
  pending
- **THEN** the loop SHALL emit `TurnDone` and terminate successfully
