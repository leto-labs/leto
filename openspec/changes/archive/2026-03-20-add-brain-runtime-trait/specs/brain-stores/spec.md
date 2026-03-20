# brain-stores Delta Spec

## ADDED Requirements

### Requirement: Stores Emit Lifecycle Events
The concrete stores SHALL emit live project and session lifecycle events.

The initial lifecycle event surface SHALL cover:

- project create, update, delete
- session create, update, delete

The initial lifecycle event surface SHALL NOT cover:

- message append events
- credential mutation events

#### Scenario: In-memory store emits session lifecycle events
- **WHEN** a caller creates, updates, or deletes a session in `InMemoryStore`
- **THEN** the store SHALL emit the corresponding session lifecycle event

#### Scenario: File store emits session lifecycle events
- **WHEN** a caller creates, updates, or deletes a session in `FileStore`
- **THEN** the store SHALL emit the corresponding session lifecycle event

#### Scenario: Project deletion emits cascaded session deletion events
- **WHEN** a project deletion removes sessions owned by that project
- **THEN** the store SHALL emit session-deleted events for those sessions
- **AND** emit the project-deleted event
