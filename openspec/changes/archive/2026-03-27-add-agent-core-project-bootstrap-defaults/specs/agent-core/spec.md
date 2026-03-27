## ADDED Requirements

### Requirement: Agent Core Restores Project Bootstrap Behavior

The `agent-core` crate SHALL restore the project bootstrap behavior required for
legacy parity on the refactored stack.

This SHALL include:

- project-root normalization
- project-local `.agents/config.toml` loading
- default provider/model/loop resolution
- project prompt or AGENTS-derived prompt fallback when no explicit override is
  present

#### Scenario: Core resolves project bootstrap from project-local config

- **WHEN** a caller resolves or creates a project through the native core
- **THEN** the core SHALL load the parity-critical defaults from the project
  tree
- **AND** use them to seed future session behavior

#### Scenario: Core falls back to project prompt guidance when session prompt is absent

- **WHEN** a caller starts a session without an explicit prompt override
- **THEN** the core SHALL resolve the effective prompt baseline from project
  defaults or project guidance rather than defaulting to an empty prompt state

#### Scenario: Core persists the resolved project bootstrap defaults

- **WHEN** the native core creates a new project from a filesystem root
- **THEN** it SHALL persist the resolved prompt, loop, provider, model, and
  runtime defaults on the stored project record
