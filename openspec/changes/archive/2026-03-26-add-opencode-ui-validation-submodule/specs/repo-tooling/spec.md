# repo-tooling Delta Spec

## ADDED Requirements

### Requirement: OpenCode UI Validation Uses A Pinned Submodule
The repository SHALL keep the runnable OpenCode UI validation consumer as a Git
submodule under `submodules/opencode`.

That submodule SHALL use the synced `leto-labs/opencode` fork as its remote,
but it SHALL be pinned to the exact commit corresponding to the compatibility
release currently used by the repository's OpenCode reference material rather
than to a floating branch tip.

#### Scenario: Contributor checks out the OpenCode UI validation target
- **WHEN** a contributor initializes repository submodules
- **THEN** the OpenCode validation consumer SHALL appear under
  `submodules/opencode`
- **AND** it SHALL resolve to the exact pinned compatibility commit rather than
  to a moving `dev` branch

#### Scenario: Compatibility pin is updated later
- **WHEN** the repository intentionally updates its OpenCode compatibility
  target
- **THEN** the submodule pin SHALL move together with that approved target
- **AND** the change SHALL remain explicit in Git history

### Requirement: OpenCode UI Validation Must Stay Release-Aligned
The repository SHALL keep the OpenCode UI validation submodule aligned with the
same approved OpenCode release used by the compatibility contract artifacts and
implemented compat work until a deliberate compatibility upgrade is approved.

#### Scenario: Contributor proposes following upstream dev
- **WHEN** a contributor proposes moving the validation submodule to a newer
  fork or upstream branch tip without also updating the repository's approved
  compatibility target
- **THEN** that proposal SHALL be rejected
- **AND** the submodule SHALL remain pinned to the release-aligned commit

#### Scenario: Repository reference pins drift apart
- **WHEN** repository-local OpenCode reference pins disagree, such as the
  submodule pin, local repocache manifest, or approved compat target
- **THEN** contributors SHALL treat that as drift to resolve
- **AND** SHALL NOT introduce a new mismatched pin for the validation submodule

### Requirement: OpenCode UI Edits Must Stay Minimal
Local changes to the validation submodule SHALL be allowed only for hard
blockers discovered during compatibility testing, such as auth transport,
CORS, or platform-specific networking seams.

Those local changes SHALL NOT be used as a substitute for missing server-side
compatibility behavior.

#### Scenario: Contributor proposes a frontend workaround for a missing compat route
- **WHEN** a frontend patch is proposed mainly because `agent-server` is
  missing required compatibility behavior
- **THEN** that patch SHALL be treated as invalid
- **AND** the required fix SHALL remain on the server-side compatibility layer

#### Scenario: Contributor patches the UI for a real transport seam
- **WHEN** a narrowly scoped UI patch is needed to address a true auth, CORS,
  or platform transport blocker
- **THEN** that patch MAY be accepted
- **AND** the change SHALL stay explicit, minimal, and documented
