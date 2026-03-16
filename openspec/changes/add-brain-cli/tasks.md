# Tasks: add-brain-cli

## Implementation Checklist

### Crate setup
- [ ] Create `crates/brain-cli/` with Cargo.toml (binary crate)
- [ ] Add to workspace members in root Cargo.toml
- [ ] Configure feature gates: openai (default), openai-oauth, mistralrs, llamacpp
- [ ] Add clap dependency for CLI parsing

### Unified provider builder
- [ ] Implement `build_provider()` that combines API + local + OAuth providers
- [ ] Auto-discover API providers from env vars (current cli-echo logic)
- [ ] Auto-discover API providers from config file
- [ ] Conditionally include local providers when features enabled
- [ ] Conditionally include OAuth provider when tokens exist
- [ ] Route all through ProviderRouter with configurable default
- [ ] Fall back to MockProvider when nothing is available

### Config integration
- [ ] Use Brain::discover(cwd) when available (depends on add-config-system)
- [ ] Fall back to env vars + defaults when no config exists
- [ ] Support AGENTS.md as system prompt

### Session persistence
- [ ] Default to FileStore (configurable directory, default `.agents/sessions/`)
- [ ] Support session listing (brain sessions list)
- [ ] Support session resumption (brain sessions resume <id>)

### CLI subcommands
- [ ] Default command: interactive TUI chat (brain or brain chat)
- [ ] brain login openai — OAuth browser flow (feature-gated on openai-oauth)
- [ ] brain login openai --device — OAuth device flow for headless
- [ ] brain logout openai — clear stored tokens
- [ ] brain sessions list — show past sessions with titles and dates
- [ ] brain sessions resume <id> — resume an existing session
- [ ] brain acp — start ACP JSON-RPC server on stdin/stdout (feature-gated on acp)
- [ ] brain serve --port <port> — start HTTP REST + SSE server (feature-gated on http)

### Remove examples
- [ ] Verify brain-cli covers all cli-echo use cases
- [ ] Verify brain-cli covers all cli-local use cases
- [ ] Remove examples/cli-echo/ directory
- [ ] Remove examples/cli-local/ directory
- [ ] Remove both from workspace Cargo.toml members
- [ ] Update README to reference brain-cli instead of examples

### Tests
- [ ] Test: default provider selection with env vars
- [ ] Test: config-driven provider selection
- [ ] Test: session list/resume with FileStore
- [ ] Test: graceful fallback when no providers available
