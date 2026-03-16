# brain-cli Delta Spec

## Note
The `brain-cli` migration described here is a planned implementation and is not present in the current workspace.

## ADDED Requirements

### Requirement: Unified Binary
The system SHALL provide a `brain-cli` binary crate that replaces both `cli-echo` and `cli-local`. It SHALL be a single entry point for all interactive brain usage, combining cloud API providers, local inference providers, and OAuth subscription providers into one binary with feature gates.

#### Scenario: Cloud provider usage
- **WHEN** `brain` is run with `OPENAI_API_KEY` set
- **THEN** it SHALL behave equivalently to the former `cli-echo` (interactive chat with OpenAI)

#### Scenario: Local provider usage
- **WHEN** `brain` is compiled with `--features llamacpp` and `MODEL=qwen3.5-0.8b`
- **THEN** it SHALL behave equivalently to the former `cli-local` (interactive chat with local model)

#### Scenario: No providers available
- **WHEN** no API keys are set, no OAuth tokens stored, and no local features enabled
- **THEN** it SHALL fall back to MockProvider with a clear message

### Requirement: Config-Driven Initialization
The `brain-cli` SHALL use the config system (from `add-config-system`) to auto-discover project configuration. When `.agents/config.toml` exists, it SHALL use it to configure providers, models, tools, and agent settings. When no config exists, it SHALL fall back to environment variables and sensible defaults.

#### Scenario: Config-driven startup
- **WHEN** `.agents/config.toml` exists in the project
- **THEN** `brain-cli` SHALL load provider, model, and tool settings from it

#### Scenario: No config fallback
- **WHEN** no `.agents/config.toml` exists
- **THEN** `brain-cli` SHALL discover providers from environment variables (same as former cli-echo)

#### Scenario: AGENTS.md as system prompt
- **WHEN** `AGENTS.md` exists at the project root
- **THEN** its content SHALL be used as part of the system prompt

### Requirement: Unified Provider Builder
The binary SHALL build a `ProviderRouter` that combines all available provider sources:
1. API providers discovered from env vars or config (OpenAI presets)
2. Local providers when feature-enabled (mistralrs, llamacpp)
3. OAuth providers when stored tokens exist (openai-oauth)
4. MockProvider as fallback

#### Scenario: Mixed cloud and local
- **WHEN** `OPENAI_API_KEY` is set and `--features mistralrs` is enabled
- **THEN** the router SHALL include both cloud and local providers, routable by model name

#### Scenario: OAuth provider auto-loaded
- **WHEN** stored OAuth tokens exist for OpenAI
- **THEN** the router SHALL include `OpenAiOAuthProvider` alongside any API-key providers

### Requirement: Session Persistence
The binary SHALL use `FileStore` by default for session persistence, so conversations survive process restarts. The store directory SHALL default to `.agents/sessions/` (relative to project root) and be configurable via config.

#### Scenario: Sessions persist
- **WHEN** a conversation happens and the process exits
- **THEN** the session SHALL be recoverable on next run via `brain sessions resume`

#### Scenario: Configurable store directory
- **WHEN** config sets `session.directory = "/tmp/brain-sessions"`
- **THEN** the FileStore SHALL use that directory

### Requirement: CLI Subcommands
The binary SHALL use `clap` for argument parsing and support the following subcommands:

- (default / `chat`) — start or resume an interactive chat session
- `login <provider>` — run OAuth login flow for a provider
- `login <provider> --device` — headless device code flow
- `logout <provider>` — clear stored OAuth tokens for a provider
- `sessions list` — list past sessions with ID, title, and date
- `sessions resume <id>` — resume an existing session by ID

#### Scenario: Interactive chat
- **WHEN** `brain` is run with no subcommand
- **THEN** it SHALL start an interactive chat session (new session by default)

#### Scenario: Login flow
- **WHEN** `brain login openai` is run
- **THEN** it SHALL start the OpenAI OAuth browser flow and store tokens on success

#### Scenario: Session listing
- **WHEN** `brain sessions list` is run
- **THEN** it SHALL print all stored sessions with their IDs, titles, and timestamps

#### Scenario: Session resume
- **WHEN** `brain sessions resume abc123` is run
- **THEN** it SHALL load the session history and start an interactive chat continuing that session

### Requirement: Feature Gates
The binary SHALL support the following feature gates:
- `openai` (default) — OpenAI-compatible API providers
- `openai-oauth` — OpenAI OAuth/subscription provider + login/logout commands
- `mistralrs` — mistral.rs local inference
- `llamacpp` — llama.cpp local inference

#### Scenario: Minimal build
- **WHEN** compiled with `--no-default-features`
- **THEN** only MockProvider SHALL be available

#### Scenario: Full build
- **WHEN** compiled with `--features openai,openai-oauth,mistralrs,llamacpp`
- **THEN** all provider types SHALL be available

### Requirement: Remove Example Binaries
The `cli-echo` and `cli-local` directories under `examples/` SHALL be removed from the workspace. Their functionality is fully subsumed by `brain-cli`. The workspace `Cargo.toml` SHALL no longer list them as members.

#### Scenario: Examples removed
- **WHEN** the workspace is built
- **THEN** `cli-echo` and `cli-local` SHALL not be present as build targets
- **AND** `brain-cli` SHALL be the only binary crate
