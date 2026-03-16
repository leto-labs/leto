# Tasks: add-brain-cli

## Implementation Checklist

### Crate setup
- [x] Create `crates/brain-cli/` with Cargo.toml (binary crate)
- [x] Add to workspace members in root Cargo.toml
- [x] Configure feature gates: openai-oauth (default), mistralrs, llamacpp
- [x] Add clap dependency for CLI parsing

### Unified provider builder
- [x] Implement `build_provider()` that combines API + local + OAuth providers
- [x] Auto-discover API providers from stored credentials via CredentialPool
- [x] Auto-discover API providers from config file
- [ ] Conditionally include local providers when features enabled
- [x] Conditionally include OAuth provider when tokens exist
- [x] Route all through ProviderRouter with configurable default
- [x] Fall back to MockProvider when nothing is available

### Config integration
- [x] Use `brain_config::resolve_fs_config(cwd)` for project config discovery
- [x] Global `~/.brain/config.toml` as base config layer
- [x] Support AGENTS.md as system prompt

### Session persistence
- [x] Default to FileStore rooted at `brain_home()` (~/.brain/)
- [x] Support session listing (brain sessions list)
- [x] Support session resumption (brain sessions resume <id>)

### CLI subcommands
- [x] Default command: interactive chat (brain)
- [x] brain credentials login openai-oauth — OAuth browser flow (feature-gated)
- [x] brain credentials login openai-oauth --device — OAuth device flow for headless
- [x] brain credentials add <provider> <api-key> — store API key
- [x] brain credentials list — list stored credentials
- [x] brain credentials remove <provider> <id> — remove credential
- [x] brain sessions list — show past sessions with titles and dates
- [x] brain sessions resume <id> — resume an existing session
- [ ] brain acp — start ACP JSON-RPC server on stdin/stdout (feature-gated on acp)
- [ ] brain serve --port <port> — start HTTP REST + SSE server (feature-gated on http)

### BrainServer integration
- [x] Bootstrap BrainServer at startup
- [x] Use `server.client()` (Arc<dyn BrainApi>) for all CLI operations
- [x] Find-or-create project based on cwd

### Remove examples
- [x] Remove examples/cli-echo/ directory
- [x] Remove examples/cli-local/ directory
- [x] Remove both from workspace Cargo.toml members
- [ ] Update README to reference brain-cli instead of examples

### Tests
- [x] Test: credential-driven provider selection
- [x] Test: config-driven provider selection
- [ ] Test: session list/resume with FileStore
- [x] Test: graceful fallback when no providers available
