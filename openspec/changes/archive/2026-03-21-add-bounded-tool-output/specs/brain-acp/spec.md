## ADDED Requirements

### Requirement: ACP Stdio Keeps Protocol Output Separate From Logs

The real and mock ACP stdio binaries SHALL keep protocol output isolated from
human-readable tracing and backend logs.

#### Scenario: ACP backend logs an internal error
- **WHEN** the real or mock ACP binary emits tracing or backend error logs
- **THEN** those logs SHALL go to stderr rather than stdout
- **AND** ACP stdout SHALL remain parseable protocol output
