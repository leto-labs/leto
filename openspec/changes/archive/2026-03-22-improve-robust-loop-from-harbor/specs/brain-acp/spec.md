## MODIFIED Requirements

### Requirement: ACP Stdio Keeps Protocol Output Separate From Logs

The real and mock ACP stdio binaries SHALL keep protocol output isolated from
human-readable tracing and backend logs.

The real ACP backend SHALL also avoid surfacing normal loop termination
conditions as ACP internal protocol errors.

The backend SHALL treat at least these loop outcomes as non-internal:

- `MaxIterations`
- `Cancelled`

#### Scenario: Loop budget exhaustion is not surfaced as ACP internal error

- **WHEN** the real ACP backend ends a turn because the loop exhausted its
  iteration budget
- **THEN** the prompt request SHALL complete normally rather than returning ACP
  `internal_error`
- **AND** the loop termination reason SHALL remain visible through the emitted
  event stream

#### Scenario: Cancelled turn is not surfaced as ACP internal error

- **WHEN** the real ACP backend ends a turn because the session was cancelled
- **THEN** the prompt request SHALL complete with ACP's cancelled stop reason
- **AND** it SHALL NOT be surfaced as ACP `internal_error`
