## ADDED Requirements

### Requirement: Real Backend Exposes ACP Session Modes

The real `agent-acp` backend SHALL expose ACP `modes` on `session/new` and
`session/load`.

The real backend SHALL derive those ACP session modes from the registered
runtime loop surface so clients can switch the active loop through the standard
ACP mode mechanism.

The backend SHALL implement `session/set_mode` and emit
`CurrentModeUpdate` when the active mode changes.

The backend SHALL NOT expose that same loop-selection surface as a duplicate
session config option.

#### Scenario: New session exposes loop-backed ACP modes

- **WHEN** ACP creates a real backend session
- **AND** one or more loops are registered
- **THEN** the response SHALL include ACP `modes`
- **AND** the current mode SHALL match the session's effective loop

#### Scenario: Setting the ACP mode updates the effective loop

- **WHEN** ACP sets the real backend session mode to a supported mode ID
- **THEN** the backend SHALL persist the corresponding session loop override
- **AND** emit `CurrentModeUpdate`

#### Scenario: Loop selection is only exposed through ACP modes

- **WHEN** ACP creates or loads a real backend session
- **THEN** the response SHALL expose loop selection through ACP `modes`
- **AND** the config option list SHALL NOT contain a duplicate `mode` or `loop`
  selector

### Requirement: Real Backend Advertises Supported ACP Commands

The real `agent-acp` backend SHALL advertise a concrete list of supported ACP
commands through `AvailableCommandsUpdate`.

The advertised commands SHALL correspond to command names that the backend can
accept from prompt text without client-specific behavior.

#### Scenario: New session emits available commands

- **WHEN** ACP creates a real backend session
- **THEN** the backend SHALL emit `AvailableCommandsUpdate`
- **AND** the advertised commands SHALL include the real backend's built-in ACP
  command set

### Requirement: Real Backend Exposes ACP Session Models

The real `agent-acp` backend SHALL expose ACP `models` on `session/new`,
`session/load`, and `session/resume` when model catalog data is available from
the current core surface.

The backend SHALL implement `session/set_model` and persist the chosen session
model override.

The backend MAY continue to expose `model` as a session config option for
client compatibility, but ACP `models` SHALL be the dedicated first-class model
surface.

#### Scenario: New session exposes ACP models

- **WHEN** ACP creates a real backend session whose current provider exposes
  one or more models
- **THEN** the response SHALL include ACP `models`

#### Scenario: Setting the ACP model persists across reload

- **WHEN** ACP sets the real backend session model to a supported model ID
- **THEN** the backend SHALL persist the corresponding session model override
- **AND** a later `session/load` SHALL report that model as current

### Requirement: Real Backend Supports ACP Session Resume Fork And Close

The real `agent-acp` backend SHALL implement the optional ACP
`session/resume`, `session/fork`, and `session/close` methods.

The backend SHALL:

- resume a session without replaying prior transcript updates
- fork a session into a new session that copies transcript and session
  overrides
- treat close as cancellation plus ACP-local resource cleanup without deleting
  persisted session history

#### Scenario: Resume returns current state without replaying transcript

- **WHEN** ACP resumes a real backend session
- **THEN** the response SHALL include the current ACP modes, models, and config
  options
- **AND** the backend SHALL NOT replay prior transcript message updates

#### Scenario: Fork copies transcript and session overrides

- **WHEN** ACP forks a real backend session
- **THEN** the new session SHALL preserve the source transcript
- **AND** preserve the source session overrides such as model, loop, and
  request config

#### Scenario: Close cancels active work but preserves stored history

- **WHEN** ACP closes a real backend session with an active turn
- **THEN** the active prompt SHALL complete as cancelled
- **AND** the stored session and transcript SHALL remain loadable afterwards

### Requirement: Real Backend Lists Sessions Beyond Cwd-Scoped Queries

The real `agent-acp` backend SHALL return all known sessions from
`session/list` when no `cwd` filter is provided.

When a `cwd` filter is provided, the backend SHALL keep the current
project-scoped behavior.

#### Scenario: List sessions without cwd returns all known sessions

- **WHEN** ACP lists sessions without providing `cwd`
- **THEN** the backend SHALL return every known session whose project root is
  available as an absolute path

### Requirement: Real Backend Preserves ACP Prompt Baseline And Session Updates

The real `agent-acp` backend SHALL honor ACP baseline prompt support for
`ResourceLink` content and SHALL emit session metadata and config updates when
backend-owned session state changes.

The backend SHALL:

- preserve `ResourceLink` content instead of collapsing it to unsupported text
- emit `SessionInfoUpdate` when the backend assigns the first prompt-derived
  session title
- emit `ConfigOptionUpdate` after supported session config mutations

#### Scenario: Prompt resource links survive ACP prompt conversion

- **WHEN** ACP prompts the real backend with `Text` and `ResourceLink` content
- **THEN** the backend SHALL preserve the resource-link reference in the
  runtime prompt text instead of replacing it with unsupported-content text

#### Scenario: First prompt title assignment emits session info update

- **WHEN** the real backend assigns a title from the first user prompt
- **THEN** it SHALL emit `SessionInfoUpdate` with the new title and timestamp

#### Scenario: Config mutation emits config option update

- **WHEN** ACP changes a supported session config option or session model
- **THEN** the backend SHALL emit `ConfigOptionUpdate` with the refreshed
  config option snapshot

### Requirement: ACP Launch Uses A Dedicated Default Agent Home

The local ACP launch surfaces SHALL default to a dedicated agent home path
instead of silently reusing the legacy `.brain` root.

If the selected store root contains an incompatible legacy layout, the launch
path SHALL fail with an actionable error that identifies the store root and the
need to set `AGENT_HOME` or choose a clean path.

#### Scenario: ACP launch defaults to the dedicated agent home

- **WHEN** a user launches `agent-acp` or `agent acp`
- **AND** neither `AGENT_HOME` nor `BRAIN_HOME` is set
- **THEN** the ACP launch path SHALL default to the dedicated agent home root

#### Scenario: Incompatible legacy store yields actionable startup failure

- **WHEN** ACP launch targets a store root whose `sessions/` directory contains
  legacy incompatible entries
- **THEN** startup SHALL fail before ACP handshake
- **AND** the error SHALL identify the incompatible root and remediation
