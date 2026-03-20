# brain-acp-integration Specification

## ADDED Requirements

### Requirement: Nori Fork Workspace For ACP Client Development

The repository SHALL provide a checked-out Nori fork workspace for ACP client
integration work at `submodules/nori-cli`.

The fork workspace SHALL:

- use `git@github.com:leto-labs/nori-cli.git` as its origin remote
- be available for local development and validation of ACP client changes
- reserve `feat/add-session-config` as the first branch for ACP session-config
  work

#### Scenario: Contributor initializes the repo workspace

- **WHEN** a contributor checks out the repository with submodules
- **THEN** the repo SHALL provide the Nori fork under `submodules/nori-cli`
- **AND** that workspace SHALL target the configured fork remote

#### Scenario: Contributor starts the scoped Nori ACP work

- **WHEN** a contributor begins the first planned ACP client branch
- **THEN** the branch name SHALL be `feat/add-session-config`

### Requirement: Session Config Options Are The First Planned ACP Client Surface

The first planned extension to the Nori ACP client SHALL target session config
options before ACP session modes.

The planned first branch SHALL:

- support ACP `configOptions`
- support `session/set_config_option`
- handle session config refreshes from ACP updates
- defer ACP session modes from the initial implementation scope

#### Scenario: Maintainer reviews initial ACP client scope

- **WHEN** the scoped Nori ACP work is reviewed in this repository
- **THEN** session config options SHALL be the first planned feature area
- **AND** ACP session modes SHALL remain deferred from that initial branch

### Requirement: Planned Nori Session Settings Stay Generic

The first planned Nori ACP session-settings surface SHALL be generic over the
session config options exposed by the ACP agent.

The planned first implementation SHALL:

- support select-style ACP session config options
- use option metadata supplied by the ACP agent
- avoid hardcoding `brain`-specific option identifiers
- use a dedicated session-settings surface instead of changing `/model` or
  reusing `/config`

#### Scenario: ACP agent exposes configurable session options

- **WHEN** a future Nori ACP session exposes select-style config options
- **THEN** the planned client surface SHALL render those options generically
- **AND** it SHALL not depend on `brain`-specific config IDs

#### Scenario: Existing Nori commands are preserved in first branch scope

- **WHEN** the initial Nori ACP session-config work is implemented
- **THEN** `/model` SHALL remain unchanged in that first branch
- **AND** `/config` SHALL remain reserved for Nori's existing local settings
