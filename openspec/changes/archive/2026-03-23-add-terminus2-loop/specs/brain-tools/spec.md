## ADDED Requirements

### Requirement: Native Terminal Session Tool

The native tool registry SHALL provide a stateful terminal-session tool for
 persistent shell interaction.

The tool SHALL support:

- session-scoped persistent shell state
- verbatim keystroke injection
- bounded waiting before output capture
- timeout reporting with current terminal state
- incremental output retrieval across loop iterations

#### Scenario: Session-scoped command preserves shell state

- **WHEN** the tool is called multiple times for the same session
- **THEN** later calls SHALL observe filesystem and shell state created by
  earlier calls

#### Scenario: Timed wait returns current terminal state

- **WHEN** the tool is invoked with keystrokes and a bounded wait interval
- **THEN** it SHALL return the current terminal state after the wait interval
- **AND** if the command exceeds the wait budget it SHALL return a timeout-style
  observation rather than tearing down the session

### Requirement: Native Tools Preset Includes Terminal Session Tool

The native tool preset SHALL include the terminal-session tool in addition to
 the existing native tools.

#### Scenario: Native preset registers terminal session tool

- **WHEN** `native_tools()` is constructed
- **THEN** the preset SHALL include the terminal-session tool
