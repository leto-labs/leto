## ADDED Requirements

### Requirement: Refactored Stack Provides Runtime-Native Terminal-Session Parity

The refactored stack SHALL provide terminal-session behavior that restores the
user-visible behavior of the legacy terminal session tooling without requiring a
duplicate public terminal tool surface on top of `agent-runtime`.

This SHALL include:

- stable terminal-session identity across calls
- command execution against an existing session
- explicit lifecycle operations such as close or equivalent cleanup
- typed runtime-facing contracts suitable for advanced loop orchestration and
  default local core assembly

#### Scenario: Caller reuses a terminal session across multiple tool calls

- **WHEN** a model or caller opens a terminal session and later issues more
  terminal operations against that same session
- **THEN** the refactored runtime SHALL preserve the session identity and route
  those operations to the same live terminal context

#### Scenario: Advanced loops depend on runtime-native terminal sessions

- **WHEN** a caller uses advanced terminal-oriented loop strategies on the
  refactored stack
- **THEN** those loops SHALL execute through `agent-runtime` PTY/session
  surfaces and typed loop/runtime contracts rather than a legacy-style
  terminal-session tool wrapper
