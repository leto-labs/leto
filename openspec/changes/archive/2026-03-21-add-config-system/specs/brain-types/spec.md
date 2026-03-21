# brain-types Delta Spec

## ADDED Requirements

### Requirement: ProjectConfig Struct
The system SHALL define a `ProjectConfig` struct in `brain-types` that represents
the resolved configuration for a project. It is pure data with no I/O or
discovery logic. It SHALL wrap `AgentConfig` via `#[serde(flatten)]`.

#### Scenario: Default config
- **WHEN** `ProjectConfig::default()` is called
- **THEN** it SHALL return a config with sensible agent defaults

#### Scenario: Serialization round-trip
- **WHEN** a `ProjectConfig` is serialized to JSON and deserialized back
- **THEN** it SHALL produce an identical `ProjectConfig`

### Requirement: Project Entity In Store
The `Project` struct SHALL include a `config: ProjectConfig` field so callers
load project configuration through the store-backed project record rather than a
separate config abstraction.

#### Scenario: Project includes config
- **WHEN** `store.projects().create(project.id, project)` is called
- **THEN** the stored project SHALL retain its `config`

### Requirement: Filesystem Config Resolution Utility
The system SHALL provide a `resolve_fs_config(root: &Path)` utility function in
`brain-config` for filesystem-based clients.

The function SHALL:

1. walk upward from the given path looking for `.agents/config.toml`
2. stop discovery at a `.git` boundary or the filesystem root
3. parse TOML into the supported config shape
4. interpolate `$ENV_VAR` and `${ENV_VAR}` in string values
5. apply layered precedence from defaults to global `~/.brain/config.toml` to
   project `.agents/config.toml`

#### Scenario: Config in current directory
- **WHEN** `.agents/config.toml` exists in the given directory
- **THEN** `resolve_fs_config(dir)` SHALL use that file

#### Scenario: Config in parent directory
- **WHEN** `.agents/config.toml` does not exist in the given directory but exists in a parent
- **THEN** discovery SHALL walk up and use the first match

#### Scenario: No config found
- **WHEN** no `.agents/config.toml` exists in any ancestor directory
- **THEN** `resolve_fs_config()` SHALL return a default `ProjectConfig`

#### Scenario: Stop at project boundary
- **WHEN** a `.git` directory is found during upward walk
- **THEN** discovery SHALL stop at that boundary

#### Scenario: String interpolation uses environment variables
- **WHEN** a config string contains `$BRAIN_VAR` or `${BRAIN_VAR}`
- **AND** the environment variable is set
- **THEN** the resolved config SHALL contain the interpolated value

#### Scenario: Missing env var is an error
- **WHEN** a config string references an unset environment variable
- **THEN** resolution SHALL return an error indicating the missing variable

### Requirement: Initial Agent Config Schema
The shipped `.agents/config.toml` schema SHALL support agent-focused settings
through:

- `[agent]`
- `[agent.inference]`

Richer provider, tools, session, and MCP sections are out of scope for this
change.

#### Scenario: Agent config parses
- **WHEN** `.agents/config.toml` contains supported `[agent]` and `[agent.inference]` fields
- **THEN** `resolve_fs_config()` SHALL parse and merge those values into `ProjectConfig`

### Requirement: AGENTS Discovery Helpers
The system SHALL provide runtime-facing AGENTS helpers in `brain-config`:

- `load_root_agents_md(root)` for `<root>/.agents/AGENTS.md`
- `list_nested_agents(project_root, cwd)` for `AGENTS.md` files between the
  project root and current working directory

These helpers are discovery utilities. They do not themselves mutate config.

#### Scenario: Root agents file loads from .agents
- **WHEN** `<project-root>/.agents/AGENTS.md` exists
- **THEN** `load_root_agents_md(project_root)` SHALL return its contents

#### Scenario: Nested agents chain is ordered from root to cwd
- **WHEN** `AGENTS.md` files exist on directories between the project root and cwd
- **THEN** `list_nested_agents(project_root, cwd)` SHALL return them in root-to-cwd order

#### Scenario: Out-of-project cwd returns no nested agents
- **WHEN** `cwd` is not under `project_root`
- **THEN** `list_nested_agents(project_root, cwd)` SHALL return an empty list
