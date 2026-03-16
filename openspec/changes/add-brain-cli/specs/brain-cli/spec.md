# brain-cli Delta Spec

## Note
The `brain-cli` crate is implemented and present in the workspace at `crates/brain-cli/`.

## ADDED Requirements

### Requirement: Unified Binary
The system SHALL provide a `brain-cli` binary crate that replaces both `cli-echo` and `cli-local`. It SHALL be a single entry point for all interactive brain usage, combining cloud API providers, local inference providers, and OAuth subscription providers into one binary with feature gates.

#### Scenario: Cloud provider usage
- **WHEN** `brain` is run with stored API credentials for an OpenAI-compatible provider
- **THEN** it SHALL use those credentials for interactive chat

#### Scenario: Local provider usage
- **WHEN** `brain` is compiled with `--features llamacpp` and a local model is configured
- **THEN** it SHALL use the local provider for interactive chat

#### Scenario: No providers available
- **WHEN** no credentials are stored, no OAuth tokens exist, and no local features enabled
- **THEN** it SHALL fall back to MockProvider with a clear message

### Requirement: Credential-Driven Provider Discovery
The `brain-cli` SHALL NOT read API keys from environment variables. Instead, providers are discovered from stored credentials (`CredentialStore`) and project config. The `build_provider()` function resolves credentials from a `CredentialPool` that wraps the store.

#### Scenario: Config-driven startup
- **WHEN** `.agents/config.toml` exists in the project
- **THEN** `brain-cli` SHALL load provider, model, and tool settings from it

#### Scenario: Global config layer
- **WHEN** `~/.brain/config.toml` exists
- **THEN** its settings SHALL be applied as a base layer (project config overrides on conflicts)

#### Scenario: No config fallback
- **WHEN** no `.agents/config.toml` or global `config.toml` exists
- **THEN** `brain-cli` SHALL discover providers from stored credentials and sensible defaults

#### Scenario: AGENTS.md as system prompt
- **WHEN** `.agents/AGENTS.md` exists at the project root
- **THEN** its content SHALL be used as part of the system prompt

### Requirement: Thin Client Over BrainServer
The `brain-cli` SHALL bootstrap a `BrainServer` once at startup and use `server.client()` (`Arc<dyn BrainApi>`) for all subsequent operations. The CLI is a thin client — it does not access `FileStore`, `Brain`, or `CredentialPool` directly after initialization.

#### Scenario: All operations via BrainApi
- **WHEN** any CLI command is executed
- **THEN** it SHALL operate through the `BrainApi` trait (project CRUD, session CRUD, credential CRUD, message streaming)

### Requirement: Unified Provider Builder
The binary SHALL build a `ProviderRouter` that combines all available provider sources:
1. API providers discovered from stored credentials via `CredentialPool`
2. Local providers when feature-enabled (mistralrs, llamacpp)
3. OAuth providers when stored OAuth tokens exist (openai-oauth)
4. MockProvider as fallback

#### Scenario: OAuth provider auto-loaded
- **WHEN** stored OAuth tokens exist for OpenAI
- **THEN** the router SHALL include `OpenAiOAuthProvider` alongside any API-key providers

### Requirement: Global Storage
The binary SHALL use `FileStore` rooted at `brain_home()` (`~/.brain/` by default, overridable via `$BRAIN_HOME`) for all persistence — projects, sessions, messages, and credentials. This is a global store shared across all projects.

#### Scenario: Sessions persist
- **WHEN** a conversation happens and the process exits
- **THEN** the session SHALL be recoverable on next run via `brain sessions resume`

#### Scenario: Global store location
- **WHEN** `brain` starts
- **THEN** it SHALL use `brain_home()` as the `FileStore` root (not a project-local `.agents/` directory)

### Requirement: CLI Subcommands
The binary SHALL use `clap` for argument parsing and support the following subcommands:

- (default) — start a new interactive chat session for the current project
- `credentials add <provider> <api-key>` — store an API key credential
- `credentials login <provider>` — run OAuth browser flow (or `--device` for device code flow)
- `credentials list` — list all stored credentials
- `credentials remove <provider> <id>` — remove a stored credential
- `sessions list` — list past sessions with ID, title, and date
- `sessions resume <id>` — resume an existing session by ID

#### Scenario: Interactive chat
- **WHEN** `brain` is run with no subcommand
- **THEN** it SHALL start an interactive chat session (new session by default)

#### Scenario: Login flow
- **WHEN** `brain credentials login openai-oauth` is run
- **THEN** it SHALL start the OpenAI OAuth browser flow and store tokens on success

#### Scenario: Session listing
- **WHEN** `brain sessions list` is run
- **THEN** it SHALL print all stored sessions with their IDs, titles, and timestamps

#### Scenario: Session resume
- **WHEN** `brain sessions resume <ULID>` is run
- **THEN** it SHALL load the session history and start an interactive chat continuing that session

### Requirement: Feature Gates
The binary SHALL support the following feature gates:
- `openai-oauth` (default) — OpenAI OAuth/subscription provider + credentials login commands
- `mistralrs` — mistral.rs local inference
- `llamacpp` — llama.cpp local inference

#### Scenario: Minimal build
- **WHEN** compiled with `--no-default-features`
- **THEN** only MockProvider SHALL be available

#### Scenario: Full build
- **WHEN** compiled with `--features openai-oauth,mistralrs,llamacpp`
- **THEN** all provider types SHALL be available

### Requirement: Remove Example Binaries
The `cli-echo` and `cli-local` directories under `examples/` SHALL be removed from the workspace. Their functionality is fully subsumed by `brain-cli`. The workspace `Cargo.toml` SHALL no longer list them as members.

#### Scenario: Examples removed
- **WHEN** the workspace is built
- **THEN** `cli-echo` and `cli-local` SHALL not be present as build targets
- **AND** `brain-cli` SHALL be the only user-facing binary crate (the `server-example` remains as a developer example)
