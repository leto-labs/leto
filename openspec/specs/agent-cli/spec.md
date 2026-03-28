# agent-cli Specification

## Purpose
Runtime-backed local CLI surface for `agent`, including interactive chat,
session and credential management, and an ACP compatibility launch path.
## Requirements
### Requirement: Unified Binary
The system SHALL provide an `agent-cli` binary crate that replaces both
`cli-echo` and `cli-local`. It SHALL be the single user-facing local CLI entry
point for interactive chat, session management, and credential management,
exposed through the `agent` command.

#### Scenario: Cloud provider usage
- **WHEN** `agent` is run with stored API credentials for an OpenAI-compatible provider
- **THEN** it SHALL use those credentials for interactive chat

#### Scenario: Local provider usage
- **WHEN** `agent-cli` is built with a local backend feature and a local model is configured
- **THEN** it SHALL use the local provider for interactive chat

#### Scenario: No providers available
- **WHEN** no credentials are stored, no OAuth tokens exist, and no local features are enabled
- **THEN** it SHALL fall back to `MockProvider` with a clear message

### Requirement: Embedded Core Bootstrap
The `agent-cli` SHALL bootstrap an embedded `AgentCoreNative` once at
startup and then operate through `Arc<dyn AgentCore>`.

It SHALL:

- resolve or create the current project through `core.resolve_or_create_project(...)`
- run turns through `core.turn(...)`
- use `core.store()` for session, message, and credential CRUD

#### Scenario: Interactive chat runs through AgentCore
- **WHEN** `agent` is run with no subcommand
- **THEN** it SHALL create or load a session through the runtime-backed store
- **AND** execute each prompt through `AgentCore::turn(...)`

#### Scenario: Session commands use runtime-backed store access
- **WHEN** `agent sessions list` or `agent sessions resume <id>` is run
- **THEN** the CLI SHALL operate through `core.store()`

### Requirement: Credential-Driven Provider Discovery
The `agent-cli` SHALL NOT read provider API keys directly from environment
variables during runtime bootstrap. Instead, providers are discovered from
stored credentials plus project/global config.

The provider bootstrap SHALL include:

1. API-key providers discovered from stored credentials
2. OAuth providers discovered from stored OAuth credentials when enabled
3. Local providers when feature-enabled
4. `MockProvider` as fallback

#### Scenario: Config-driven startup
- **WHEN** `.agents/config.toml` exists in the project
- **THEN** `agent-cli` SHALL load provider, model, and tool settings from it

#### Scenario: Global config layer
- **WHEN** `~/.agent/config.toml` exists
- **THEN** its settings SHALL be applied as a base layer

#### Scenario: AGENTS.md as system prompt
- **WHEN** `.agents/AGENTS.md` exists at the project root
- **THEN** its content SHALL be used as part of the system prompt

### Requirement: Global Storage
The binary SHALL use `FileStore` rooted at `agent_home()` (`~/.agent/` by
default) for projects, sessions, messages, and credentials.

#### Scenario: Sessions persist
- **WHEN** a conversation happens and the process exits
- **THEN** the session SHALL be recoverable on the next run via `agent sessions resume`

#### Scenario: Global store location
- **WHEN** `agent` starts
- **THEN** it SHALL use `agent_home()` as the `FileStore` root

### Requirement: CLI Commands
The binary SHALL use `clap` for argument parsing and support the currently
implemented command surface:

- default interactive chat (`agent`)
- `credentials add <provider> <api-key>`
- `credentials login <provider>`
- `credentials list`
- `credentials remove <provider> <id>`
- `sessions list`
- `sessions resume <id>`
- `acp`

#### Scenario: Login flow
- **WHEN** `agent credentials login openai-oauth` is run
- **THEN** it SHALL start the OpenAI OAuth flow and store tokens on success

#### Scenario: Session listing
- **WHEN** `agent sessions list` is run
- **THEN** it SHALL print stored sessions with their IDs, titles, and timestamps

#### Scenario: Session resume
- **WHEN** `agent sessions resume <ULID>` is run
- **THEN** it SHALL load the session history and continue that session

#### Scenario: ACP alias
- **WHEN** `agent acp` is run
- **THEN** it SHALL start the ACP stdio compatibility path exposed by `agent-acp`

### Requirement: Feature Gates
The binary SHALL support the following feature gates:

- `openai-oauth` (default) — OpenAI OAuth/subscription provider and login command
- `mistralrs` — mistral.rs local inference
- `llamacpp` — llama.cpp local inference

#### Scenario: Minimal build
- **WHEN** compiled with `--no-default-features`
- **THEN** only the non-OAuth provider set and mock fallback SHALL be available

#### Scenario: Full build
- **WHEN** compiled with `--features openai-oauth,mistralrs,llamacpp`
- **THEN** all supported provider types SHALL be available

### Requirement: Example Binaries Removed
The `cli-echo` and `cli-local` directories under `examples/` SHALL be removed
from the workspace. Their functionality is subsumed by `agent-cli`.

#### Scenario: Examples removed
- **WHEN** the workspace is built
- **THEN** `cli-echo` and `cli-local` SHALL not be present as build targets
- **AND** `agent-cli` SHALL be the user-facing local binary crate

### Requirement: Remote Modes Deferred
Remote CLI modes SHALL NOT be part of the current implementation.

#### Scenario: Serve and attach remain future work
- **WHEN** evaluating the implemented `agent-cli`
- **THEN** `agent serve` and `agent attach` SHALL be treated as deferred work
- **AND** their design SHALL follow the future remote `AgentCore` path rather than a legacy `BrainApi`-style surface

### Requirement: Non-Interactive Exec Command

The `agent` CLI SHALL support a non-interactive `exec` command for benchmark
and harness use.

The command SHALL:

- accept explicit `cwd`, `provider`, `model`, `loop`, and `output_dir`
- accept an explicit API key for the selected provider
- default to the Responses API when the selected OpenAI-compatible provider
  preset supports it
- allow the API surface to be explicitly overridden for debugging or
  compatibility
- execute one user instruction through the embedded runtime
- write structured run artifacts to the requested output directory

#### Scenario: Exec command writes Harbor-facing artifacts

- **WHEN** `agent exec` runs successfully
- **THEN** it SHALL write `run.json`, `events.jsonl`, and ATIF `trajectory.json`
- **AND** it SHALL exit successfully

#### Scenario: Exec command emits native ATIF trajectory

- **WHEN** Harbor runs the direct `agent` surface
- **THEN** `agent exec` SHALL emit Harbor-compatible ATIF `trajectory.json`
- **AND** Harbor-facing usage and cost metrics SHALL be sourced from `run.json`

#### Scenario: Exec command bypasses global agent home

- **WHEN** `agent exec` is used for a Harbor run
- **THEN** it SHALL NOT depend on stored `agent credentials`
- **AND** it SHALL NOT require `~/.agent/config.toml`
- **AND** it SHALL resolve only project-local config plus explicit CLI inputs

### Requirement: Exec Runtime Exposes Terminus2 Loop

The native CLI/runtime bootstrap SHALL register `terminus2` as a selectable
 loop name anywhere common runtime loops are registered.

#### Scenario: Exec can select terminus2

- **WHEN** a caller runs `agent exec --loop terminus2`
- **THEN** the runtime SHALL resolve and execute `Terminus2Loop`

### Requirement: Exec Runtime Exposes TerminusKira Loop

The native CLI/runtime bootstrap SHALL register `terminus-kira` as a selectable
loop name anywhere common runtime loops are registered.

#### Scenario: Exec can select terminus-kira

- **WHEN** a caller runs `agent exec --loop terminus-kira`
- **THEN** the runtime SHALL resolve and execute `TerminusKiraLoop`
