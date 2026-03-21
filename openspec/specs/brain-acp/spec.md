# brain-acp Specification

## Purpose
ACP stdio compatibility surface for `brain`. Defines the real and mock ACP
entrypoints, runtime-backed session handling, ACP-facing session
configuration, and protocol-safe stdio behavior for external ACP clients.
## Requirements
### Requirement: ACP Backend Depends On BrainRuntime
The real ACP backend SHALL depend on the shared `BrainRuntime` boundary rather
than on `Brain` plus `ProviderRouter`.

The backend SHALL:

- hold a runtime trait object in app state
- use `runtime.resolve_or_create_project()` for cwd-to-project bootstrap
- use `runtime.turn()` and `runtime.cancel_turn()` for active turn lifecycle
- use `runtime` model helper methods for session-model inspection and updates
- use `runtime.store()` for plain session/message/project CRUD

#### Scenario: Real backend uses runtime for project and turn handling
- **WHEN** ACP creates or prompts a real session
- **THEN** it SHALL resolve the project through `BrainRuntime`
- **AND** execute and cancel turns through `BrainRuntime`

#### Scenario: Real backend uses runtime model helpers
- **WHEN** ACP inspects or updates the current session model
- **THEN** it SHALL use the runtime model helper methods rather than `ProviderRouter`

### Requirement: Real Backend Exposes ACP Config Options

The real `brain-acp` backend SHALL expose ACP `configOptions` for session
settings.

The real surface SHALL include:

- `model`
- `thought_level`
- `loop` when more than one loop is registered

The backend SHALL include those config options on both `session/new` and
`session/load`.

#### Scenario: Real backend omits loop config when no alternative exists
- **WHEN** ACP creates or loads a real backend session with only one registered loop
- **THEN** the response SHALL omit the `loop` config option

#### Scenario: Real backend exposes loop config when multiple loops are registered
- **WHEN** ACP creates or loads a real backend session with multiple registered loops
- **THEN** the response SHALL include a `loop` config option

### Requirement: Real Backend Supports Session Config Mutation

The real `brain-acp` backend SHALL implement `session/set_config_option`.

The backend SHALL:

- accept `model` updates
- accept `thought_level` updates when supported by the effective model
- accept `loop` updates when the named loop is registered
- return the full current config-option snapshot after each update

#### Scenario: Loop update returns full config options
- **WHEN** ACP sets the real backend session `loop`
- **THEN** the backend SHALL persist the session loop override
- **AND** return the full current `configOptions`

### Requirement: Real Backend Groups Model Options By Provider

The real backend `model` config option SHALL be exposed as a provider-grouped
select.

#### Scenario: Model config option preserves provider grouping
- **WHEN** ACP inspects real backend config options
- **THEN** the `model` option SHALL group choices by provider
- **AND** preserve the current runtime model ordering within each provider group

### Requirement: ACP Stdio Keeps Protocol Output Separate From Logs

The real and mock ACP stdio binaries SHALL keep protocol output isolated from
human-readable tracing and backend logs.

#### Scenario: ACP backend logs an internal error
- **WHEN** the real or mock ACP binary emits tracing or backend error logs
- **THEN** those logs SHALL go to stderr rather than stdout
- **AND** ACP stdout SHALL remain parseable protocol output

### Requirement: Real ACP Session Model State

When ACP unstable session model support is enabled, the real ACP backend SHALL
report model state from the real `brain-core` session/runtime model rather than
from ACP-local adapter state.

#### Scenario: New session reports effective model state

- **WHEN** a client creates a real ACP session
- **THEN** the response SHALL include the current effective model derived from the session and project config
- **AND** it SHALL include the available unambiguous models exposed by the provider router

#### Scenario: Load session reports persisted model state

- **WHEN** a client loads a real ACP session
- **THEN** the response SHALL include the current persisted session model state

### Requirement: Real ACP Session Model Switching

When ACP unstable session model support is enabled, the real ACP backend SHALL
support `session/set_model` through persisted session inference overrides.

#### Scenario: session/set_model persists provider and model on the session

- **WHEN** ACP `session/set_model` is called with a valid model ID
- **THEN** the real ACP backend SHALL resolve the owning provider through `ProviderRouter`
- **AND** it SHALL persist provider+model on the real `Session.inference`

#### Scenario: session/set_model preserves other session inference fields

- **WHEN** a session already has per-session `temperature` or `max_tokens` overrides
- **THEN** changing only the model through ACP SHALL preserve those other session-local overrides

### Requirement: Dedicated ACP Binaries

The system SHALL expose the ACP surface directly from the `brain-acp` crate
through dedicated binaries rather than only through `brain-cli`.

The ACP binary identities SHALL be:

- `brain-acp`
- `brain-acp-mock`

#### Scenario: Mock ACP is launched directly from brain-acp

- **WHEN** a user or ACP client launches the explicit mock ACP binary
- **THEN** the system SHALL start the ACP stdio server without going through the `brain` compatibility alias

#### Scenario: Real and mock ACP binaries remain distinct

- **WHEN** ACP launch paths are configured for local validation
- **THEN** `brain-acp` SHALL represent the real runtime-backed backend
- **AND** `brain-acp-mock` SHALL remain the explicit mock backend

### Requirement: Explicit Mock Binary Identity

The system SHALL keep the mock ACP runtime available through an explicit
mock-scoped binary identity.

#### Scenario: Mock ACP remains available for development

- **WHEN** developers need ACP interoperability validation against mock behavior
- **THEN** they SHALL be able to launch `brain-acp-mock`

#### Scenario: Compatibility harness uses explicit mock path

- **WHEN** the repo runs its external ACP compatibility harness
- **THEN** the harness SHALL launch the explicit mock ACP binary rather than relying on `brain-cli`

### Requirement: Temporary brain ACP Compatibility Alias

The system SHALL keep `brain acp` only as a temporary compatibility path
while direct `brain-acp` binaries become the canonical ACP entrypoints.

#### Scenario: Existing brain ACP command continues to work

- **WHEN** a user launches `brain acp`
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

#### Scenario: Nori entries can target distinct real and mock binaries

- **WHEN** Nori registers both local ACP entries
- **THEN** one entry SHALL be able to target `brain-acp`
- **AND** the other SHALL be able to target `brain-acp-mock`

### Requirement: Real Backend Can Bridge File Reads And Writes Through ACP Clients
The real `brain-acp` backend SHALL be able to route file reads and writes
through ACP client-owned filesystem capabilities when running under an ACP
client that provides them.

This path SHALL allow containerized ACP clients such as Harbor to keep file
edits inside the client-managed task workspace rather than writing only to the
backend host filesystem.

#### Scenario: Harbor-backed ACP file operations land in the task workspace
- **WHEN** the real `brain-acp` backend is launched through Harbor's ACP client
- **AND** the model uses the `file_write` or `file_read` tool with a relative path
- **THEN** the file operation SHALL be sent through the ACP client bridge
- **AND** the resolved path SHALL be rooted in the ACP session cwd exposed by the client

