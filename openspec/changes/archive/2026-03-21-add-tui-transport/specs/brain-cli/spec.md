# brain-cli Delta Spec

## Note
This change currently describes planned TUI behavior. The `brain-cli` crate is
implemented in the workspace, but the TUI frontend described here is not.
This delta is archived as superseded by the ACP-first product direction and is
not promoted into canonical specs.

## MODIFIED Requirements

### Requirement: Unified Binary
The system SHALL provide a `brain-cli` binary crate as the main interactive
frontend for `brain`. When the `tui` feature is enabled, running `brain` with
no subcommand SHALL launch the TUI in local embedded-runtime mode.

The TUI is a runtime-backed app surface. It does NOT implement the `Transport`
trait.

#### Scenario: Default local TUI
- **WHEN** `brain` is run with the `tui` feature enabled and no subcommand
- **THEN** it SHALL start the TUI against an embedded runtime

#### Scenario: Engine boundary remains runtime-backed
- **WHEN** the TUI is running
- **THEN** it SHALL communicate with the engine through the runtime boundary and
  runtime-backed store access

### Requirement: CLI Subcommands
The binary SHALL support the existing provider, credential, and session
commands. Remote `serve` / `attach` modes remain deferred to the separate
server/runtime proposal.

#### Scenario: Remote modes stay deferred during local TUI MVP
- **WHEN** evaluating the scope of the TUI MVP
- **THEN** `brain serve` and `brain attach` SHALL remain out of scope for this change

### Requirement: Feature Gates
The feature gate list SHALL be extended with:
- `tui` (default) — ratatui-based terminal UI

#### Scenario: Default build includes TUI
- **WHEN** `brain-cli` is compiled with default features
- **THEN** the TUI frontend SHALL be available

## ADDED Requirements

### Requirement: TuiApp
The system SHALL provide a `TuiApp` that acts as a thin local client of the
runtime boundary and renders an interactive full-screen terminal UI using
ratatui + crossterm.

#### Scenario: Full-screen app startup
- **WHEN** `TuiApp` starts
- **THEN** it SHALL enter the alternate screen, enable raw mode, and render the TUI

#### Scenario: Clean terminal restore
- **WHEN** `TuiApp` exits or panics
- **THEN** the terminal SHALL be restored to its prior state

### Requirement: TUI Layout
The TUI SHALL render four persistent regions:

1. `Header`
2. `Transcript`
3. `Composer`
4. `Footer`

The layout SHALL remain usable on narrow terminals by truncating ambient
metadata before sacrificing transcript or composer space.

#### Scenario: Layout on startup
- **WHEN** the TUI opens a session
- **THEN** the user SHALL see header, transcript, composer, and footer

#### Scenario: Terminal resize
- **WHEN** the terminal is resized
- **THEN** the layout SHALL re-render without losing transcript, input, or session state

### Requirement: State-Driven Interaction Model
The TUI SHALL model at least the following runtime states:

- `idle`
- `drafting`
- `running`
- `running_with_queued_input`
- `tool_running`
- `interrupted`
- `loading`
- `error`

The composer and footer SHALL change behavior and rendering based on these
states while the transcript remains the stable primary surface.

#### Scenario: Idle state
- **WHEN** no turn is active and the composer is empty
- **THEN** the footer SHALL show idle operational hints

#### Scenario: Running state
- **WHEN** assistant output is streaming
- **THEN** the footer SHALL show a spinner and interrupt hint
- **AND** the transcript SHALL continue updating in place

#### Scenario: Running with queued input
- **WHEN** a turn is active and the user submits another message
- **THEN** the TUI SHALL queue that message locally
- **AND** the queue state SHALL be visible in the composer or footer area

### Requirement: Transcript Rendering
The transcript SHALL be scrollable and transcript-first.

- user messages render distinctly from assistant/tool activity
- assistant text streams incrementally into the active transcript item
- basic markdown structure SHALL be rendered readably
- when the user is scrolled away from the bottom, new content SHALL NOT force-jump the view

#### Scenario: Streaming assistant message
- **WHEN** token events arrive
- **THEN** the active assistant transcript item SHALL update incrementally

#### Scenario: User scrolled up
- **WHEN** new transcript content arrives while the user is reading older content
- **THEN** the scroll position SHALL remain stable
- **AND** the UI SHALL indicate that newer content exists below

### Requirement: Inline Tool Activity
Tool activity SHALL render inline in the transcript rather than in a separate
tool pane.

- light tools MAY render as single-line status entries
- verbose or long-running tools SHALL render as compact blocks

#### Scenario: Running tool
- **WHEN** a tool starts during a turn
- **THEN** the transcript SHALL show that tool inline with a running state

#### Scenario: Tool completes
- **WHEN** a tool finishes
- **THEN** the existing transcript item SHALL update with its completed state and summarized result

### Requirement: Composer and Input Handling
The composer SHALL be multiline and optimized for interactive chat.

- `Enter` submits
- modified enter inserts newline
- input history SHALL be available
- while a turn is active, the user MAY keep drafting
- while a turn is active, submitting a new message SHALL queue it for the next turn

#### Scenario: Multiline draft
- **WHEN** the user inserts newlines in the composer
- **THEN** the composer SHALL grow up to its configured maximum height before scrolling internally

#### Scenario: Queued follow-up message
- **WHEN** a turn is active and the user submits another message
- **THEN** the message SHALL be queued locally instead of rejected

### Requirement: Footer and Status UX
The footer SHALL act as a context-sensitive control and status surface.

- idle: help/session/model hints
- drafting: submit/edit hints
- running: spinner + active-work summary + cancel hint
- queued: pending-message summary
- loading/reconnect: attach/hydration status

#### Scenario: Idle footer
- **WHEN** the TUI is waiting for user input
- **THEN** the footer SHALL show concise operational hints

#### Scenario: Busy footer
- **WHEN** a turn is active
- **THEN** the footer SHALL show a spinner, active state summary, and interrupt hint in a stable location

### Requirement: Slash Commands and Session Switching
The TUI SHALL provide a small slash-command surface:

- `/help`
- `/new`
- `/sessions`
- `/model`
- `/quit`

Session switching SHALL happen within the TUI rather than forcing the user
back to a separate CLI flow.

#### Scenario: Session list
- **WHEN** the user runs `/sessions`
- **THEN** the TUI SHALL open a session list overlay or picker

#### Scenario: Switch session
- **WHEN** the user selects a different session
- **THEN** the transcript SHALL reload that session's history

### Requirement: Cancellation UX
The TUI SHALL support interruption with preserved partial output.

#### Scenario: Escape during active turn
- **WHEN** the user presses `Escape` during an active turn
- **THEN** the TUI SHALL call `client.cancel_turn(session_id)`
- **AND** the footer SHALL show a cancelling/interrupted transition

#### Scenario: Partial output preserved
- **WHEN** a turn is interrupted
- **THEN** partial assistant text and any already-visible tool activity SHALL remain visible in the transcript

### Requirement: TUI Validation Coverage
The TUI implementation SHALL include automated validation deep enough to catch
UX regressions, not just event-plumbing regressions.

It SHALL include:

- deterministic state-transition tests
- deterministic rendering or snapshot tests
- deterministic integration tests for local runtime-backed mode
- an opt-in live-provider validation path that runs when `OPENAI_API_KEY` is available

#### Scenario: Deterministic offline validation
- **WHEN** the normal test suite is run without live provider credentials
- **THEN** the TUI SHALL still be covered by state, rendering, and integration tests that do not require network access

#### Scenario: Snapshot coverage
- **WHEN** major TUI states such as idle, running, queued, interrupted, and narrow-width layouts are rendered in tests
- **THEN** stable snapshots or equivalent rendering assertions SHALL protect those states from silent UX regressions

#### Scenario: Live provider validation
- **WHEN** `OPENAI_API_KEY` is available in the environment or loaded from `.env`
- **THEN** an opt-in live TUI validation suite SHALL be able to exercise at least one real streaming turn and one interrupted streaming turn
