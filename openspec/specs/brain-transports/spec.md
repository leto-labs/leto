# brain-transports Specification

## Purpose
IO abstraction between the engine and the outside world. Defines the `Transport` trait, `InputEvent` enum, and `CliTransport` (stdin/stdout) implementation.
## Requirements
### Requirement: InputEvent Enum
The system SHALL define an `InputEvent` enum representing user-initiated actions delivered through a transport. The initial variant SHALL be `Message(String)` for text input. Future variants (e.g., Cancel, SwitchSession) MAY be added without breaking existing transports.

#### Scenario: Text input
- **WHEN** a user types "hello" in a CLI
- **THEN** the transport SHALL produce `InputEvent::Message("hello".into())`

### Requirement: Transport Trait
The system SHALL define a `Transport` trait in `brain-types` with three methods:
- `name() -> &str` — human-readable transport identifier
- `recv() -> BoxFuture<Result<Option<InputEvent>, BrainError>>` — awaits the next user input; returns `None` on EOF/shutdown
- `send(event: Event) -> BoxFuture<Result<(), BrainError>>` — delivers a single engine event to the user

The trait SHALL be object-safe (`Send + Sync`) and usable as `&dyn Transport`.

#### Scenario: Transport receives user message
- **WHEN** `recv()` is called and the user has typed a line
- **THEN** it SHALL return `Ok(Some(InputEvent::Message(...)))`

#### Scenario: Transport signals shutdown
- **WHEN** `recv()` is called and the input source is closed (EOF)
- **THEN** it SHALL return `Ok(None)`

#### Scenario: Transport sends token event
- **WHEN** `send(Event::Token { delta })` is called
- **THEN** the transport SHALL render the delta to its output (e.g., print to stdout)

#### Scenario: Transport sends error event
- **WHEN** `send(Event::Error { .. })` is called
- **THEN** the transport SHALL render the error appropriately (e.g., print to stderr)

### Requirement: CliTransport
The system SHALL include a `CliTransport` implementation in the `brain-transports` crate. It reads lines from stdin and renders events to stdout/stderr.

- `recv()` SHALL print a prompt (e.g., `"> "`) to stderr, then read one line from stdin. Empty lines SHALL be skipped (recv again). EOF SHALL return `None`.
- `send()` SHALL handle events as follows:
  - `Token { delta }` — print delta to stdout (no newline)
  - `ToolCallStart { name, .. }` — print tool name to stdout
  - `ToolCallDone { result, .. }` — print result to stdout
  - `TurnDone { .. }` — print summary to stdout with newline
  - `Error { message, .. }` — print to stderr
  - `MessageDone { .. }` — no output (internal bookkeeping)

#### Scenario: Interactive CLI session
- **WHEN** CliTransport is used with a Brain
- **THEN** the user SHALL see a prompt, type input, see streaming tokens, and see a turn summary

#### Scenario: Piped input
- **WHEN** stdin is piped (not a TTY)
- **THEN** CliTransport SHALL still read lines and return None on EOF

#### Scenario: Empty line skipped
- **WHEN** the user presses Enter with no text
- **THEN** CliTransport SHALL re-prompt without producing an InputEvent


