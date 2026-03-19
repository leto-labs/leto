# brain-core Specification

## ADDED Requirements

### Requirement: Project Resolution By Root

The system SHALL provide a `Brain` helper for resolving or creating a project by
working directory root.

The method signature SHALL be:
`resolve_or_create_project(&self, root: PathBuf) -> Result<Project, BrainError>`

#### Scenario: Existing project is reused for matching root

- **WHEN** a project already exists with the same normalized root path
- **THEN** `resolve_or_create_project(root)` SHALL return that project

#### Scenario: Missing project is created from root

- **WHEN** no project exists for the given normalized root path
- **THEN** `resolve_or_create_project(root)` SHALL create a new project with that root and return it
