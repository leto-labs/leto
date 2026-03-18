# Nori

Source-first notes on `nori-cli` as a non-editor ACP client relevant to
`brain acp`.

This repo was inspected from the local repocache clone on 2026-03-18.

## Short Answer

`nori-cli` is the strongest true TUI candidate we have found so far for a final
multi-turn ACP UX pass against `brain acp`.

The main reason is no longer just "it looks like a good TUI". The important
source-level change is that Nori now appears to have a real custom ACP agent
story, including config schema, registry wiring, and an ignored live end-to-end
test for a local custom agent.

## What It Is

`nori-cli` is a Rust/Ratatui terminal client built around ACP subprocesses. The
repo is a fork-style workspace, with the actual implementation living under
`codex-rs/`.

Relevant paths:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/agent.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/agent.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/docs.md`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/docs.md)

## Custom ACP Launch

This was the main open question, and the answer is now clearly yes.

`nori-cli` has a data-driven ACP agent registry that combines built-in agents
with user-defined entries from `[[agents]]` in `config.toml`. The config schema
supports:

- local command execution
- `npx`
- `bunx`
- `pipx`
- `uvx`

That logic is explicit in:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs)

The custom-agent path is not just theoretical. There is an ignored live E2E
test that registers a local custom ACP agent through:

```toml
[[agents]]
name = "ElizACP"
slug = "elizacp"

[agents.distribution.local]
command = "elizacp"
args = ["--deterministic", "acp"]
```

and then validates startup and message exchange in the TUI:

- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs)

For `brain`, that means a plausible path exists to point Nori at direct
`brain-acp` crate binaries rather than routing ACP through `brain-cli`.

The old single-agent shape was:

```toml
[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "brain-acp", "--bin", "brain-acp-mock"]
```

The current local setup on this machine now uses two ACP entries in
`~/.nori/cli/config.toml` so both binary identities are visible in the picker:

```toml
agent = "brain-acp-mock"

[[agents]]
name = "Brain ACP"
slug = "brain-acp"

[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "brain-acp", "--bin", "brain-acp"]

[[agents]]
name = "Brain ACP Mock"
slug = "brain-acp-mock"

[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "brain-acp", "--bin", "brain-acp-mock"]
```

Today both binaries are still backed by the mock runtime. Later, only
`brain-acp` should switch to the real ACP implementation.

## ACP Coverage

Nori's ACP surface is broad and deeper than the other standalone clients we
inspected.

What is clearly covered in source/tests:

- session creation and prompt loop
- session resume and `session/load` fallback handling
- reasoning and thought chunks
- tool call rendering and lifecycle updates
- permission requests
- client `fs/read_text_file`
- synthetic file-write style tool updates
- plan translation and rendering
- agent picker and multi-agent switching
- unstable model switching support

Evidence:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/event_translation.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/event_translation.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs)

One important nuance: the current UX still thinks in terms of switching between
ACP agents more than in terms of "raw arbitrary subprocess command". So the
custom-agent path is real, but it is config-centric rather than ad hoc.

## Fit For `brain`

`nori-cli` is the strongest current TUI candidate we have found so far for a
final multi-turn ACP UX pass.

Why:

- real Ratatui UX
- explicit ACP-native architecture
- substantial ACP test coverage
- actual local custom-agent support in source

Risks:

- less trivial to configure than `acp-ui`
- custom local agents go through Nori's config and registry model, not a one-off command field
- its upstream messaging is still provider-first, so custom local agent UX may be less polished than the underlying implementation suggests
- client terminal ACP methods are currently stubbed to `method_not_found`

## `brain acp` Validation Note

One practical interoperability detail showed up during manual validation with
the current mock `brain acp` server: Nori intercepts slash-prefixed chat input
for its own TUI command layer before it reaches the ACP agent.

That means the mock server should expose its synthetic probes only through
normal chat input with a `mock:` prefix. The current shape is:

- `mock:create-plan`
- `mock:summarize-session`
- `mock:think [topic]`
- `mock:search [query]`
- `mock:fetch [resource]`
- `mock:edit-file [/absolute/path]`
- `mock:delete-file [/absolute/path]`
- `mock:move-file [/absolute/from /absolute/to]`
- `mock:read-file [/absolute/path]`
- `mock:write-file [/absolute/path] [content]`
- `mock:request-permission`
- `mock:terminal [command] [args...]`
- `mock:terminal-kill [command] [args...]`

This keeps the same mock ACP flows reachable from Nori's normal chat input.

Most of these commands now support omitted arguments and fall back to
deterministic defaults. That makes quick iteration in Nori easier:

- `mock:read-file` defaults to a likely session-root file such as `Cargo.toml`
- `mock:write-file` defaults to a synthetic `mock-output.txt`
- `mock:edit-file`, `mock:delete-file`, and `mock:move-file` default to synthetic session-local paths
- `mock:terminal` defaults to `echo hello-from-mock-terminal`
- `mock:terminal-kill` defaults to `sleep 5`

One more nuance from the Nori validation pass: if the mock streams file or
terminal results only as assistant text, Nori will show a tool row with
`(no output)`. The mock therefore works better when these probes emit explicit
ACP `ToolCall` and `ToolCallUpdate` events with synthetic output content.

Another important limitation is that Nori's current ACP client delegate does
not implement terminal client methods. The stubs for:

- `terminal/create`
- `terminal/output`
- `terminal/release`
- `terminal/wait_for_exit`
- `terminal/kill`

currently return `method_not_found` in:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs`](../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs)

So `mock:terminal ...` and `mock:terminal-kill ...` are expected to fail in
Nori today even though they work in `acpx`. Terminal ACP behavior should be
validated with `acpx`, not treated as a `brain acp` interoperability failure
inside Nori.

## Repo-Owned Prompt Pack

To make the mock ACP flows faster to drive from Nori, this repo now keeps a
small prompt pack under:

- [`tools/nori-prompts/`](../../tools/nori-prompts)

These are installed into Nori's actual custom prompt directory:

- `~/.nori/cli/commands`

through a symlink installer:

- [`scripts/install-nori-prompts.sh`](../../scripts/install-nori-prompts.sh)

or the matching `just` recipe:

```bash
just nori-prompts-install
```

That produces slash-style Nori commands such as:

- `/prompts:mock-create-plan`
- `/prompts:mock-summarize-session`
- `/prompts:mock-think`
- `/prompts:mock-search`
- `/prompts:mock-fetch`
- `/prompts:mock-read-file`
- `/prompts:mock-write-file`
- `/prompts:mock-request-permission`
- `/prompts:mock-edit-file`
- `/prompts:mock-delete-file`
- `/prompts:mock-move-file`

These Nori custom prompts are only a shortcut layer. They expand into the
existing `mock:*` prompt text that the local `brain acp` mock agent already
understands.

The prompt pack intentionally excludes terminal commands because Nori's ACP
client still returns `method_not_found` for terminal methods. Keep using
`acpx` for:

- `mock:terminal ...`
- `mock:terminal-kill ...`

One more iteration detail: because the local Nori agents are launched through
`cargo run`, source changes are only picked up when Nori starts a fresh
conversation. After editing `brain-acp`, run `/new` before re-testing so Nori
respawns the agent process.
