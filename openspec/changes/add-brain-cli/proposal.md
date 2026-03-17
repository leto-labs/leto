# Proposal: add-brain-cli

## Why

The workspace currently has two example binaries:

- **cli-echo** (`examples/cli-echo/`) — cloud/API providers via `ProviderRouter`
  with env-var-driven `OpenAiConfigPreset` auto-discovery
- **cli-local** (`examples/cli-local/`) — local inference via `MistralRsProvider`
  or `LlamaCppProvider` with feature gates

These served their purpose as PoC demos for the SDK, but they have problems:

1. **Duplicated structure**: both do the exact same thing — build a provider,
   create Brain with InMemoryStore + SimpleLoop + native_tools + CliTransport,
   call `brain.run()`. The only difference is `build_provider()`.

2. **No config support**: both use env vars and hardcoded defaults. No `.agents/`
   config, no AGENTS.md, no session persistence.

3. **No unified experience**: a user who wants both cloud and local providers
   needs to choose which binary to run. With `ProviderRouter`, there's no
   reason these can't coexist.

4. **Blocking TUI work**: the TUI needs a real binary to live in, not a
   throwaway example.

5. **examples/ signals "not real"**: keeping them as examples suggests they're
   not the intended way to use brain. A `brain-cli` crate signals a real,
   supported entry point.

## What

### New: `brain-cli` crate

A single binary crate (initially at `crates/brain-cli/`, workspace member)
that replaces both examples:

1. **Config-driven**: uses `Brain::discover(cwd)` (from `add-config-system`)
   to auto-discover `.agents/config.toml` and `AGENTS.md`. Falls back to env
   vars and sensible defaults when no config exists.

2. **Unified provider selection**: builds a `ProviderRouter` that includes:
   - All API providers whose keys are available (from presets or config)
   - Local providers if feature-enabled (mistralrs, llamacpp)
   - OAuth providers if authenticated (from stored tokens)
   - Mock provider as ultimate fallback

3. **Session persistence**: uses `FileStore` by default (not `InMemoryStore`),
   so conversations survive restarts. Store directory configurable via config.

4. **Runtime modes** (via `clap`):
   - `brain` (default) — launch the interactive frontend
   - `brain serve` — run the HTTP + SSE server headlessly
   - `brain attach <url>` — attach the interactive frontend to a remote server
   - `brain sessions list` — list past sessions
   - `brain sessions resume <id>` — resume a past session
   - `brain credentials ...` — manage API key and OAuth credentials
   - Future: `brain config init` — scaffold `.agents/config.toml`

5. **Interactive frontend hosting**: `brain-cli` is the host binary for the
   TUI-first interactive experience tracked in `add-tui-transport`. It boots
   `BrainServer`, uses `server.client()` for local mode, and exposes the same
   server surface used by `brain attach`.

6. **Feature gates** for optional providers:
   - `openai` (default) — OpenAI-compatible API providers
   - `openai-oauth` — OpenAI OAuth/subscription provider
   - `mistralrs` — local mistral.rs inference
   - `llamacpp` — local llama.cpp inference

### Remove: `cli-echo` and `cli-local`

Both example binaries are removed from the workspace. Their functionality is
fully subsumed by `brain-cli`:

- `brain` with `OPENAI_API_KEY` set = current `cli-echo` behavior
- `brain --features llamacpp` with `MODEL=qwen3.5-0.8b` = current `cli-local`

### Migration path

No external consumers depend on cli-echo or cli-local (they're internal
examples). The migration is:

1. Create `brain-cli` with unified provider logic
2. Verify `brain-cli` covers all cli-echo and cli-local use cases
3. Remove `examples/cli-echo/` and `examples/cli-local/` directories
4. Remove them from workspace `Cargo.toml` members
5. Update README to point to `brain-cli`

## Change Dependencies

- **Requires**: `add-config-system` (uses `Brain::discover()` for config-driven initialization)
- **Requires**: `add-server-architecture` (starts BrainServer, exposes HTTP endpoint)
- **Requires**: `add-oauth-provider` (login/logout subcommands, feature-gated)
- **Coordinates with**: `add-tui-transport` (owns detailed TUI UX and testing behavior)

## Impact

- **New crate**: `crates/brain-cli/` (binary, workspace member)
- **Removed**: `examples/cli-echo/`, `examples/cli-local/`
- **New spec**: `brain-cli`
- **Modifies**: `brain-engine` (workspace members list)
- **New dependencies**: `clap` for CLI argument parsing
