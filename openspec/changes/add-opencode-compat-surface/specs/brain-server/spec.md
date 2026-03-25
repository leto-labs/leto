# brain-server Delta Spec

## ADDED Requirements

### Requirement: OpenCode Compatibility Surface Is Secondary
The future server architecture SHALL treat any OpenCode-compatible HTTP surface
as a secondary compatibility API layered on top of Brain's canonical runtime
architecture rather than as the primary client boundary.

The compatibility surface SHALL NOT redefine the canonical architecture away
from `BrainRuntime`, `BrainRuntimeNative`, and a future `BrainRuntimeRemote`.

#### Scenario: Compatibility API is documented
- **WHEN** the future OpenCode-compatible API is proposed or implemented
- **THEN** it SHALL be described as a compatibility surface
- **AND** the canonical client boundary SHALL remain `BrainRuntime`

### Requirement: Compatibility Targets anomalyco/opencode Session-Oriented App API
The planned compatibility target SHALL be the actively maintained
`anomalyco/opencode` codebase and the session-oriented app API used by its UI
and SDK.

The planned compatibility scope SHALL include:

- project/workspace flows
- session lifecycle
- message history and message posting
- permission flows
- provider/config flows
- event streaming

The planned compatibility scope SHALL NOT include, in this change:

- PTY routes
- generic file routes
- MCP routes
- TUI-only routes
- experimental routes

#### Scenario: Scope is reviewed before implementation
- **WHEN** a contributor reviews this change to determine what the future
  compatibility layer should cover
- **THEN** the target SHALL be the session-oriented app API of
  `anomalyco/opencode`
- **AND** unrelated upstream route families SHALL remain out of scope for this
  change

### Requirement: Upstream UI Configurability Must Be Preserved
The future compatibility work SHALL preserve the upstream UI's ability to
configure and connect to custom servers wherever possible.

Frontend deltas SHALL be allowed only for hard blockers such as:

- CORS / origin handling
- auth transport differences
- Tauri or webview networking constraints
- unavoidable platform integration seams

Frontend deltas SHALL NOT be used to avoid building the compatibility adapter.

#### Scenario: Frontend delta is proposed to work around a missing endpoint
- **WHEN** a contributor proposes a frontend change mainly because the Brain
  compatibility surface is incomplete
- **THEN** that frontend change SHALL be treated as invalid for this planned
  direction
- **AND** the missing adapter work SHALL remain the required fix

#### Scenario: Frontend delta is proposed for a true transport blocker
- **WHEN** a contributor proposes a frontend change because a browser or Tauri
  transport constraint prevents the upstream UI from connecting cleanly
- **THEN** that change MAY be allowed
- **AND** it SHALL be documented as a hard-blocker exception rather than as an
  adapter substitute

### Requirement: OpenCode Compatibility Work Is Hard-Blocked
The planned OpenCode-compatible surface SHALL NOT be implemented until its
runtime, store, hosting, and mapping prerequisites are complete and approved.

The blockers SHALL include:

- approved remote-runtime parity work around `BrainRuntime`
- stabilized store semantics for project, session, message, credential, and
  trajectory behavior
- a hosted `BrainRuntimeNative` server model rather than legacy `BrainApi`
- a written endpoint and semantic mapping matrix for the targeted API surface
- specified auth, approval, workspace-selection, and server-mount decisions

#### Scenario: Contributor wants to start compatibility implementation early
- **WHEN** the blocker list is not yet complete and approved
- **THEN** implementation of the OpenCode-compatible surface SHALL remain
  blocked
- **AND** this change SHALL be treated as planning-only

### Requirement: Future UI Adoption Is Separate From This Change
This change SHALL record `anomalyco/opencode` as the intended future frontend
reference consumer and submodule candidate, but it SHALL NOT require submodule
adoption or frontend vendoring as part of this change.

#### Scenario: Contributor reads this change looking for submodule instructions
- **WHEN** a contributor reviews this change
- **THEN** they SHALL understand that upstream UI adoption is a future decision
- **AND** this change SHALL not itself require adding or updating a submodule
