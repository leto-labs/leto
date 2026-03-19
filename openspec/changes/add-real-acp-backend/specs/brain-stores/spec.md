# brain-stores Specification

## ADDED Requirements

### Requirement: FileStore And InMemoryStore Support Project Lookup By Root

The built-in store implementations SHALL support project lookup by normalized
root path.

#### Scenario: InMemoryStore finds project by root

- **WHEN** an in-memory project exists with the requested normalized root
- **THEN** `InMemoryStore` SHALL return it from `project_find_by_root`

#### Scenario: FileStore finds project by root

- **WHEN** a stored project file exists with the requested normalized root
- **THEN** `FileStore` SHALL return it from `project_find_by_root`
