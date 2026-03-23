## ADDED Requirements

### Requirement: Exec Runtime Exposes Terminus2 Loop

The native CLI/runtime bootstrap SHALL register `terminus2` as a selectable
 loop name anywhere common runtime loops are registered.

#### Scenario: Exec can select terminus2

- **WHEN** a caller runs `brain exec --loop terminus2`
- **THEN** the runtime SHALL resolve and execute `Terminus2Loop`
