# brain-cli Delta Spec

## Note
The `brain-cli` crate is implemented and present in the workspace at
`crates/brain-cli/`.

## ADDED Requirements

### Requirement: Unified Binary
The system SHALL provide a `brain-cli` binary crate that replaces both
`cli-echo` and `cli-local`. It SHALL be the single user-facing local CLI entry
point for interactive chat, session management, and credential management.

#### Scenario: Cloud provider usage
- **WHEN** `brain` is run with stored API credentials for an OpenAI-compatible provider
- **THEN** it SHALL use those credentials for interactive chat

#### Scenario: Local provider usage
- **WHEN** `brain-cli` is built with a local backend feature and a local model is configured
- **THEN** it SHALL use the local provider for interactive chat

#### Scenario: No providers available
- **WHEN** no credentials are stored, no OAuth tokens exist, and no local features are enabled
- **THEN** it SHALL fall back to `MockProvider` with a clear message

### Requirement: Embedded Runtime Bootstrap
The `brain-cli` SHALL bootstrap an embedded `BrainRuntimeNative` once at
startup and then operate through `Arc<dyn BrainRuntime>`.

It SHALL:

- resolve or create the current project through `runtime.resolve_or_create_project(...)`
- run turns through `runtime.turn(...)`
- use `runtime.store()` for session, message, and credential CRUD

#### Scenario: Interactive chat runs through BrainRuntime
- **WHEN** `brain` is run with no subcommand
- **THEN** it SHALL create or load a session through the runtime-backed store
- **AND** execute each prompt through `BrainRuntime::turn(...)`

#### Scenario: Session commands use runtime-backed store access
- **WHEN** `brain sessions list` or `brain sessions resume <id>` is run
- **THEN** the CLI SHALL operate through `runtime.store()`

### Requirement: Credential-Driven Provider Discovery
The `brain-cli` SHALL NOT read provider API keys directly from environment
variables during runtime bootstrap. Instead, providers are discovered from
stored credentials plus project/global config.

The provider bootstrap SHALL include:

1. API-key providers discovered from stored credentials
2. OAuth providers discovered from stored OAuth credentials when enabled
3. Local providers when feature-enabled
4. `MockProvider` as fallback

#### Scenario: Config-driven startup
- **WHEN** `.agents/config.toml` exists in the project
- **THEN** `brain-cli` SHALL load provider, model, and tool settings from it

#### Scenario: Global config layer
- **WHEN** `~/.brain/config.toml` exists
- **THEN** its settings SHALL be applied as a base layer

#### Scenario: AGENTS.md as system prompt
- **WHEN** `.agents/AGENTS.md` exists at the project root
- **THEN** its content SHALL be used as part of the system prompt

### Requirement: Global Storage
The binary SHALL use `FileStore` rooted at `brain_home()` (`~/.brain/` by
default) for projects, sessions, messages, and credentials.

#### Scenario: Sessions persist
- **WHEN** a conversation happens and the process exits
- **THEN** the session SHALL be recoverable on the next run via `brain sessions resume`

#### Scenario: Global store location
- **WHEN** `brain` starts
- **THEN** it SHALL use `brain_home()` as the `FileStore` root

### Requirement: CLI Commands
The binary SHALL use `clap` for argument parsing and support the currently
implemented command surface:

- default interactive chat (`brain`)
- `credentials add <provider> <api-key>`
- `credentials login <provider>`
- `credentials list`
- `credentials remove <provider> <id>`
- `sessions list`
- `sessions resume <id>`
- `acp`

#### Scenario: Login flow
- **WHEN** `brain credentials login openai-oauth` is run
- **THEN** it SHALL start the OpenAI OAuth flow and store tokens on success

#### Scenario: Session listing
- **WHEN** `brain sessions list` is run
- **THEN** it SHALL print stored sessions with their IDs, titles, and timestamps

#### Scenario: Session resume
- **WHEN** `brain sessions resume <ULID>` is run
- **THEN** it SHALL load the session history and continue that session

#### Scenario: ACP alias
- **WHEN** `brain acp` is run
- **THEN** it SHALL start the ACP stdio compatibility path exposed by `brain-acp`

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
from the workspace. Their functionality is subsumed by `brain-cli`.

#### Scenario: Examples removed
- **WHEN** the workspace is built
- **THEN** `cli-echo` and `cli-local` SHALL not be present as build targets
- **AND** `brain-cli` SHALL be the user-facing local binary crate

### Requirement: Remote Modes Deferred
Remote CLI modes SHALL NOT be part of the current implementation.

#### Scenario: Serve and attach remain future work
- **WHEN** evaluating the implemented `brain-cli`
- **THEN** `brain serve` and `brain attach` SHALL be treated as deferred work
- **AND** their design SHALL follow the future remote `BrainRuntime` path rather than `BrainApi`
