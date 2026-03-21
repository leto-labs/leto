# brain-types Specification

## ADDED Requirements

### Requirement: Project Lookup By Root

The store abstraction SHALL support looking up a project by normalized root
path.

`ProjectStore` SHALL provide:
`project_find_by_root(&self, root: &Path) -> Result<Option<Project>, BrainError>`

#### Scenario: Store finds project by root

- **WHEN** a store contains a project whose root matches the requested normalized path
- **THEN** `project_find_by_root(root)` SHALL return that project

#### Scenario: Store misses unknown root

- **WHEN** no stored project root matches the requested normalized path
- **THEN** `project_find_by_root(root)` SHALL return `None`
