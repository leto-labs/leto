# repo-tooling Delta Spec

## ADDED Requirements

### Requirement: Repository Maintains An OpenCode UI Validation Matrix

The repository SHALL maintain a source-driven validation matrix for the pinned
OpenCode UI consumer under `submodules/opencode`.

That matrix SHALL define the user-visible interactions that later compatibility
validation and automation must cover when exercising `agent-server` through the
OpenCode-compatible surface.

#### Scenario: Contributors need to understand validation scope
- **WHEN** a contributor plans or reviews OpenCode compatibility work
- **THEN** the repository SHALL provide a maintained validation matrix for the
  pinned OpenCode UI consumer
- **AND** that matrix SHALL describe the expected interaction coverage beyond a
  minimal “UI loads” smoke check

### Requirement: OpenCode Validation Matrix Must Be Source-Driven And Release-Aligned

The OpenCode validation matrix SHALL stay aligned with the same pinned OpenCode
release used by the validation submodule and compatibility reference material.

The matrix SHALL be derived from the pinned OpenCode app, generated SDK, and
follow-up runtime or server source review in repocache rather than from memory
or assumptions alone.

The primary acceptance target for this matrix SHALL be pinned OpenCode web UI
compatibility, not full SDK or TUI parity.

#### Scenario: Deeper repocache pass reveals missed interactions
- **WHEN** contributors perform a deeper pass through the pinned OpenCode
  runtime or server modules and discover additional UI-relevant interactions
- **THEN** the matrix SHALL be updated to include those interactions before a
  later implementation claims complete validation coverage

### Requirement: OpenCode Validation Matrix Must Cover Critical And Full Interaction Sets

The validation matrix SHALL classify interactions into a later critical smoke
set and a later broader regression set.

For each interaction, the matrix SHALL record the UI entrypoint, underlying
SDK methods, compat routes, required events, expected state changes, and
visible UI outcome.

The matrix SHALL explicitly capture multi-step flows whose implementation
depends on more than one route, such as provider credential mutation and
workspace reset or cleanup, rather than flattening them into a single vague
interaction label.

#### Scenario: Later automation is scoped from the matrix
- **WHEN** contributors implement or extend later OpenCode automation
- **THEN** they SHALL be able to derive both must-pass smoke flows and broader
  regression flows from the maintained matrix
- **AND** the matrix SHALL be detailed enough to trace each flow back to the
  underlying compat surfaces

#### Scenario: Non-web product surfaces are reviewed
- **WHEN** contributors perform deeper repocache review and discover
  non-web-only OpenCode surfaces
- **THEN** those surfaces SHALL be reflected in the matrix only if they affect
  browser-visible behavior
- **AND** otherwise they SHALL be explicitly marked as outside the current
  web-validation lane
