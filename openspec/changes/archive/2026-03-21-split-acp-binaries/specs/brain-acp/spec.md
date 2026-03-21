# brain-acp Specification

## ADDED Requirements

### Requirement: Dedicated ACP Binaries

The system SHALL expose the ACP surface directly from the `brain-acp` crate
through dedicated binaries rather than only through `brain-cli`.

The ACP binary identities SHALL be:

- `brain-acp`
- `brain-acp-mock`

#### Scenario: Mock ACP is launched directly from brain-acp

- **WHEN** a user or ACP client launches the explicit mock ACP binary
- **THEN** the system SHALL start the ACP stdio server without requiring `brain-cli`

#### Scenario: Product ACP binary name exists before real backend

- **WHEN** the real ACP backend is not implemented yet
- **THEN** the `brain-acp` binary MAY temporarily share the mock runtime while preserving its future binary identity

### Requirement: Explicit Mock Binary Identity

The system SHALL keep the mock ACP runtime available through an explicit
mock-scoped binary identity.

#### Scenario: Mock ACP remains available for development

- **WHEN** developers need ACP interoperability validation against mock behavior
- **THEN** they SHALL be able to launch `brain-acp-mock`

#### Scenario: Compatibility harness uses explicit mock path

- **WHEN** the repo runs its external ACP compatibility harness
- **THEN** the harness SHALL launch the explicit mock ACP binary rather than relying on `brain-cli`

### Requirement: Temporary brain-cli ACP Compatibility Alias

The system SHALL keep `brain-cli -- acp` only as a temporary compatibility path
while direct `brain-acp` binaries become the canonical ACP entrypoints.

#### Scenario: Existing brain-cli ACP command continues to work

- **WHEN** a user launches `brain-cli -- acp`
- **THEN** the ACP stdio server SHALL still start successfully

#### Scenario: Direct brain-acp binaries are canonical

- **WHEN** ACP launch paths are documented or configured for external clients
- **THEN** direct `brain-acp` binaries SHALL be treated as the canonical ACP surface

### Requirement: Dual Nori ACP Registration

The system SHALL support registering both ACP binary identities in Nori at the
same time.

#### Scenario: Nori registers both ACP agents

- **WHEN** Nori is configured for local `brain` ACP validation
- **THEN** it SHALL be able to register one entry for `brain-acp` and one entry for `brain-acp-mock`

#### Scenario: Nori entries are temporarily mock-backed

- **WHEN** the real ACP backend does not yet exist
- **THEN** both Nori entries MAY temporarily launch mock-backed binaries until the `brain-acp` implementation is replaced
