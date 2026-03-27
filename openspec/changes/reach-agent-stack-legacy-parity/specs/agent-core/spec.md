## ADDED Requirements

### Requirement: Agent Core Restores Legacy Project Bootstrap Behavior

The `agent-core` crate SHALL restore the project bootstrap behavior required for
legacy parity on the refactored stack.

This SHALL include:

- project-root normalization
- persisted project-config loading
- default provider/model/loop resolution
- project prompt or AGENTS-derived prompt fallback when no explicit override is
  present

#### Scenario: Core resolves project bootstrap from persisted config

- **WHEN** a caller resolves or creates a project through the native core
- **THEN** the core SHALL load the persisted parity-critical project defaults
- **AND** use them to seed future session behavior

#### Scenario: Core falls back to project prompt guidance when session prompt is absent

- **WHEN** a caller starts a session without an explicit prompt override
- **THEN** the core SHALL resolve the effective prompt baseline from project
  defaults or project guidance rather than defaulting to an empty prompt state
