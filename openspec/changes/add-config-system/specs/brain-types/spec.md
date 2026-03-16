# brain-types Delta Spec

## ADDED Requirements

### Requirement: ProjectConfig Struct
The system SHALL define a `ProjectConfig` struct in `brain-types` that represents the full resolved configuration for a project. It is pure data with no I/O or discovery logic. It SHALL wrap `AgentConfig` via `#[serde(flatten)]` and serve as the extensible container for project-level settings.

Future extensions (provider routing, tool presets, MCP server declarations) will add fields to `ProjectConfig` without breaking existing code.

#### Scenario: Default config
- **WHEN** `ProjectConfig::default()` is called
- **THEN** it SHALL return a config with sensible defaults (max_iterations=20, no system prompt, no explicit model)

#### Scenario: Serialization round-trip
- **WHEN** a `ProjectConfig` is serialized to JSON and deserialized back
- **THEN** it SHALL produce an identical `ProjectConfig`

### Requirement: Project Entity in Store
The `Project` struct (defined in the store spec) SHALL include a `config: ProjectConfig` field. Project config SHALL be stored with the project so loading a project from the Store returns its config without any separate config loading abstraction.

Different platforms create projects with configs resolved from different sources:
- **CLI/TUI**: resolves config from the local filesystem via `resolve_fs_config()`
- **Web**: fetches config from an API or database
- **Tests**: uses `ProjectConfig::default()` or inline construction

All paths converge on `store.project_create(Project { config, .. })`.

#### Scenario: Project includes config
- **WHEN** `store.project_create(Project { config, .. })` is called
- **THEN** the resulting project SHALL retain `config`

### Requirement: Filesystem Config Resolution Utility
The system SHALL provide a `resolve_fs_config(root: &Path)` utility function (in a `brain-config` crate or module) for resolving project configuration from the local filesystem. This is NOT a trait — it's a plain function used by filesystem-based clients (CLI, TUI).

The function SHALL:

1. **Walk upward** from the given directory toward the filesystem root, looking for `.agents/config.toml`. Discovery stops at a project boundary (`.git` directory) or the filesystem root. First match wins.

2. **Parse `.agents/config.toml`** into `ProjectConfig` sections using the TOML format.

3. **Resolve env var interpolation**: Config values in TOML files support `$ENV_VAR` and `${ENV_VAR}` syntax.

4. **Apply layered resolution** (lowest to highest priority):
   1. Hardcoded defaults
   2. Project config file (`.agents/config.toml`)
   3. Environment variables
   4. Programmatic overrides (if any were passed)

   Higher-priority layers override lower-priority layers on a per-field basis.

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
- **THEN** discovery SHALL stop at that directory (it is the project root)

#### Scenario: API key from env
- **WHEN** the config file contains `api_key = "$OPENAI_API_KEY"`
- **AND** the env var `OPENAI_API_KEY` is set to `"sk-xxx"`
- **THEN** the resolved config SHALL have `api_key = "sk-xxx"`

#### Scenario: Missing env var
- **WHEN** a config value references `$MISSING_VAR` and the var is not set
- **THEN** resolution SHALL return an error indicating the missing variable

### Requirement: AGENTS.md Integration (Runtime)
Directory-scoped `AGENTS.md` discovery SHALL be a runtime concern, NOT a project-creation concern. Directory-scoped `AGENTS.md` files SHALL provide context-dependent instructions that vary based on where the agent is currently operating.

The agent loop / Brain SHALL discover and parse `AGENTS.md` files dynamically:
- Root-level `AGENTS.md` at the project root is prepended to the system prompt at session start
- Nested `AGENTS.md` files in subdirectories are injected as additional context when the agent operates in those directories

#### Scenario: Root AGENTS.md as system prompt
- **WHEN** an `AGENTS.md` file exists at the project root
- **THEN** its body content SHALL be prepended to the agent's system prompt

#### Scenario: AGENTS.md with frontmatter
- **WHEN** an `AGENTS.md` file contains YAML frontmatter with a `model` key
- **THEN** the frontmatter metadata SHALL be merged into the resolved config

#### Scenario: No AGENTS.md
- **WHEN** no `AGENTS.md` file exists
- **THEN** no additional system prompt context SHALL be added from this source

### Requirement: Config Schema
The `.agents/config.toml` file SHALL support the following sections:

- `[model]` — default model, model aliases, fallback chains
- `[provider]` — provider configs (API keys reference env vars via `$ENV_VAR`)
- `[tools]` — tool enable/disable, custom tool definitions, MCP server refs
- `[agent]` — system prompt, max iterations, temperature, other loop settings
- `[session]` — store backend, session directory

#### Scenario: Config schema baseline
- **WHEN** a `.agents/config.toml` includes each of the documented sections
- **THEN** `resolve_fs_config()` SHALL parse the file without schema errors


## ADDED Requirements

### Requirement: ProjectConfig Struct
The system SHALL define a `ProjectConfig` struct in `brain-types` that represents the full resolved configuration for a project. It is pure data with no I/O or discovery logic. It SHALL wrap `AgentConfig` via `#[serde(flatten)]` and serve as the extensible container for project-level settings.

Future extensions (provider routing, tool presets, MCP server declarations) will add fields to `ProjectConfig` without breaking existing code.

#### Scenario: Default config
- **WHEN** `ProjectConfig::default()` is called
- **THEN** it SHALL return a config with sensible defaults (max_iterations=20, no system prompt, no explicit model)

#### Scenario: Serialization round-trip
- **WHEN** a `ProjectConfig` is serialized to JSON and deserialized back
- **THEN** it SHALL produce an identical `ProjectConfig`

### Requirement: Project Entity in Store
The `Project` struct (defined in the store spec) SHALL include a `config: ProjectConfig` field. Project config SHALL be stored with the project so loading a project from the Store returns its config without any separate config loading abstraction.

Different platforms create projects with configs resolved from different sources:
- **CLI/TUI**: resolves config from the local filesystem via `resolve_fs_config()`
- **Web**: fetches config from an API or database
- **Tests**: uses `ProjectConfig::default()` or inline construction

All paths converge on `store.project_create(Project { config, .. })`.

#### Scenario: Project includes config
- **WHEN** `store.project_create(Project { config, .. })` is called
- **THEN** the resulting project SHALL retain `config`

### Requirement: Filesystem Config Resolution Utility
The system SHALL provide a `resolve_fs_config(root: &Path)` utility function (in a `brain-config` crate or module) for resolving project configuration from the local filesystem. This is NOT a trait — it's a plain function used by filesystem-based clients (CLI, TUI).

The function SHALL:

1. **Walk upward** from the given directory toward the filesystem root, looking for `.agents/config.toml`. Discovery stops at a project boundary (`.git` directory) or the filesystem root. First match wins.

2. **Parse `.agents/config.toml`** into `ProjectConfig` sections using the TOML format.

3. **Resolve env var interpolation**: Config values in TOML files support `$ENV_VAR` and `${ENV_VAR}` syntax.

4. **Apply layered resolution** (lowest to highest priority):
   1. Hardcoded defaults
   2. Project config file (`.agents/config.toml`)
   3. Environment variables
   4. Programmatic overrides (if any were passed)

   Higher-priority layers override lower-priority layers on a per-field basis.

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
- **THEN** discovery SHALL stop at that directory (it is the project root)

#### Scenario: API key from env
- **WHEN** the config file contains `api_key = "$OPENAI_API_KEY"`
- **AND** the env var `OPENAI_API_KEY` is set to `"sk-xxx"`
- **THEN** the resolved config SHALL have `api_key = "sk-xxx"`

#### Scenario: Missing env var
- **WHEN** a config value references `$MISSING_VAR` and the var is not set
- **THEN** resolution SHALL return an error indicating the missing variable

### Requirement: AGENTS.md Integration (Runtime)
Directory-scoped `AGENTS.md` discovery SHALL be a runtime concern, NOT a project-creation concern. Directory-scoped `AGENTS.md` files SHALL provide context-dependent instructions that vary based on where the agent is currently operating.

The agent loop / Brain SHALL discover and parse `AGENTS.md` files dynamically:
- Root-level `AGENTS.md` at the project root is prepended to the system prompt at session start
- Nested `AGENTS.md` files in subdirectories are injected as additional context when the agent operates in those directories

#### Scenario: Root AGENTS.md as system prompt
- **WHEN** an `AGENTS.md` file exists at the project root
- **THEN** its body content SHALL be prepended to the agent's system prompt

#### Scenario: AGENTS.md with frontmatter
- **WHEN** an `AGENTS.md` file contains YAML frontmatter with a `model` key
- **THEN** the frontmatter metadata SHALL be merged into the resolved config

#### Scenario: No AGENTS.md
- **WHEN** no `AGENTS.md` file exists
- **THEN** no additional system prompt context SHALL be added from this source

### Requirement: Config Schema
The `.agents/config.toml` file SHALL support the following sections:

- `[model]` — default model, model aliases, fallback chains
- `[provider]` — provider configs (API keys reference env vars via `$ENV_VAR`)
- `[tools]` — tool enable/disable, custom tool definitions, MCP server refs
- `[agent]` — system prompt, max iterations, temperature, other loop settings
- `[session]` — store backend, session directory

#### Scenario: Config schema baseline
- **WHEN** a `.agents/config.toml` includes each of the documented sections
- **THEN** `resolve_fs_config()` SHALL parse the file without schema errors
