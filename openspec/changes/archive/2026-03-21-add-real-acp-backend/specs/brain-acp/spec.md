# brain-acp Specification

## MODIFIED Requirements

### Requirement: Dedicated ACP Binaries

The system SHALL expose the ACP surface directly from the `brain-acp` crate
through dedicated binaries rather than only through `brain-cli`.

The ACP binary identities SHALL be:

- `brain-acp`
- `brain-acp-mock`

`brain-acp` SHALL be the real ACP backend entrypoint.
`brain-acp-mock` SHALL remain the explicit mock ACP backend entrypoint.

#### Scenario: Real ACP is launched directly from brain-acp

- **WHEN** a user or ACP client launches `brain-acp`
- **THEN** the system SHALL start the real ACP stdio server without requiring `brain-cli`

#### Scenario: Mock ACP remains directly launchable

- **WHEN** a user or ACP client launches `brain-acp-mock`
- **THEN** the system SHALL start the mock ACP stdio server

### Requirement: Standalone Mock ACP Surface

The system SHALL provide an ACP-compatible mock agent surface for `brain` over
stdio.

The mock ACP surface SHALL:

- live in `brain-acp`
- remain available for ACP interoperability validation
- use a mirrored internal module layout relative to the real backend so the two
  implementations are easy to compare

#### Scenario: Mock ACP backend stays available after real backend lands

- **WHEN** the real ACP backend is implemented
- **THEN** the explicit mock ACP backend SHALL still be launchable and testable

## ADDED Requirements

### Requirement: Real ACP Backend Uses brain-core

The system SHALL provide a real ACP backend in `brain-acp` that executes ACP
session lifecycle and prompt turns through `brain-core::Brain`.

The ACP SDK SHALL remain the protocol source of truth inside `brain-acp`.

#### Scenario: New ACP session creates real brain session

- **WHEN** an ACP client sends `session/new`
- **THEN** the real ACP backend SHALL resolve a `brain` project from the ACP working directory
- **AND** it SHALL create a real `brain` session through `Brain`

#### Scenario: ACP prompt uses Brain turn execution

- **WHEN** an ACP client sends `session/prompt`
- **THEN** the real ACP backend SHALL execute the turn through `Brain::turn`
- **AND** it SHALL stream ACP updates derived from emitted `brain` events

### Requirement: Real ACP Session Loading And Listing

The real ACP backend SHALL support runtime-backed session loading and listing.

#### Scenario: Load ACP session replays stored history

- **WHEN** a client sends `session/load` for a known session in the cwd-derived project
- **THEN** the ACP layer SHALL replay stored system, user, and assistant history through ACP session updates before completing the load request

#### Scenario: List ACP sessions by cwd-derived project

- **WHEN** a client sends `session/list` with a working directory filter
- **THEN** the ACP layer SHALL return only sessions that belong to the resolved `brain` project for that root

### Requirement: Real ACP Cancellation

The real ACP backend SHALL support cancelling an active `Brain::turn` for a
session.

#### Scenario: ACP cancel stops runtime-backed turn

- **WHEN** a client sends `session/cancel` for a session with an active turn
- **THEN** the real ACP backend SHALL trigger cancellation for that `Brain::turn`
- **AND** the prompt response SHALL complete with ACP `cancelled`
