## ADDED Requirements

### Requirement: Exec Runtime Exposes TerminusKira Loop

The native CLI/runtime bootstrap SHALL register `terminus-kira` as a selectable
loop name anywhere common runtime loops are registered.

#### Scenario: Exec can select terminus-kira

- **WHEN** a caller runs `brain exec --loop terminus-kira`
- **THEN** the runtime SHALL resolve and execute `TerminusKiraLoop`
