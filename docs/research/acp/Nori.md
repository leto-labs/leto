# Nori

Source-first notes on `nori-cli` as a non-editor ACP client relevant to
`agent acp`.

This repo was inspected from the local repocache clone on 2026-03-18.

## Short Answer

`nori-cli` is the strongest true TUI candidate we have found so far for a final
multi-turn ACP UX pass against `agent acp`.

The main reason is no longer just "it looks like a good TUI". The important
source-level change is that Nori now appears to have a real custom ACP agent
story, including config schema, registry wiring, and an ignored live end-to-end
test for a local custom agent.

## What It Is

`nori-cli` is a Rust/Ratatui terminal client built around ACP subprocesses. The
repo is a fork-style workspace, with the actual implementation living under
`codex-rs/`.

Relevant paths:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/agent.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/agent.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/docs.md`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/docs.md)

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

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/config/types/mod.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/registry.rs)

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

- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/live_custom_agent.rs)

For this repo, that means a plausible path exists to point Nori at direct
`agent-acp` crate binaries rather than routing ACP through `agent-cli`.

The old single-agent shape was:

```toml
[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "agent-acp", "--bin", "agent-acp-mock"]
```

The current local setup on this machine now uses two ACP entries in
`~/.nori/cli/config.toml` so both binary identities are visible in the picker:

```toml
agent = "agent-acp-mock"

[[agents]]
name = "Agent ACP"
slug = "agent-acp"

[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "agent-acp", "--bin", "agent-acp"]

[[agents]]
name = "Agent ACP Mock"
slug = "agent-acp-mock"

[agents.distribution.local]
command = "cargo"
args = ["run", "-q", "-p", "agent-acp", "--bin", "agent-acp-mock"]
```

Today both binaries are still backed by the mock runtime. Later, only
`agent-acp` should switch to the real ACP implementation.

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

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/event_translation.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/event_translation.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui-pty-e2e/tests/acp_mode.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/nori/agent_picker.rs)

One important nuance: the current UX still thinks in terms of switching between
ACP agents more than in terms of "raw arbitrary subprocess command". So the
custom-agent path is real, but it is config-centric rather than ad hoc.

## Modes And Config Gaps

ACP itself now has a native session-control story for:

- session modes
- session config options
- `session/set_config_option`

The official docs are explicit:

- Session modes: <https://agentclientprotocol.com/protocol/session-modes>
- Session config options: <https://agentclientprotocol.com/protocol/session-config-options>
- Schema: <https://agentclientprotocol.com/protocol/schema#session%2Fset-config-option>

But the current Nori ACP path inspected in this repo is materially narrower
than ACP-the-protocol.

### Confirmed In The Inspected Nori Path

- unstable `session/set_model` support is clearly wired through the ACP
  connection and TUI picker flow
- the user-visible `/model` help text already treats model and reasoning as a
  combined UX concept

Evidence:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/public_api.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/public_api.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/pickers.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/pickers.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/history_cell/mod.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/history_cell/mod.rs)

### Not Found In The Inspected Nori Path

- a generic ACP config-option fetch/render/update path in the TUI
- an ACP `session/set_config_option` client path comparable to the confirmed
  `session/set_model` path
- a clear ACP session-mode picker or mode-change UI path for external ACP
  agents

This wording is intentional. It does **not** mean those features are impossible
to add to Nori. It means they were not found in the concrete ACP client path
inspected here, so the docs should not overclaim current support.

## What This Means For `brain`

The practical architecture conclusion changed once ACP config options became
the right protocol-native place for controls like thinking level or fast mode.

If the product goal is "serious ACP-native session controls in the main TUI we
actually want to use," then the cleaner path now looks like:

- keep `agent-acp` protocol-native and runtime-clean
- treat Nori as the strongest TUI base we have
- add the missing ACP mode/config-option UX to Nori directly

That is a better boundary than teaching `agent-acp` to emulate client-specific
combined controls or overloading the model picker with reasoning/speed variants
just because the current client path is narrower than the protocol.

So the current documentation recommendation is:

- use `acpx` for baseline ACP interoperability
- use Nori as the main TUI reference
- prefer a Nori fork/extension over a `agent-acp` bridge if richer ACP session
  settings are the next product step

## Local Fork Workflow

For the current fork-based ACP work, the primary local development loop should
use Nori's existing Rust workflow in `codex-rs`:

- `just nori`
- `cargo run --bin nori --`

That is still consistent with upstream architecture. In Nori, the native Rust
binary is the real application, while the npm package is a thin launcher that
wraps vendored platform binaries. In the local fork:

- `vendor/` is a packaging concern for the npm wrapper, not a source artifact
- `dist/` is temporary packaging output, not a routine development input
- neither should be treated as part of the tracked day-to-day dev loop

This matters because the current ACP session-config work changes the Rust TUI
and ACP layers, not the npm launcher or release assembly logic. So the cleanest
way to mimic upstream architecture while keeping the patch small is:

- develop against the native binary from source
- reserve package-shape validation for a later smoke test

Relevant upstream architecture references in the fork:

- [`submodules/nori-cli/codex-rs/justfile`](../../submodules/nori-cli/codex-rs/justfile)
- [`submodules/nori-cli/nori-cli/bin/nori.js`](../../submodules/nori-cli/nori-cli/bin/nori.js)
- [`submodules/nori-cli/.github/workflows/nori-release.yml`](../../submodules/nori-cli/.github/workflows/nori-release.yml)

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

## `agent acp` Validation Note

One practical interoperability detail showed up during manual validation with
the current mock `agent acp` server: Nori intercepts slash-prefixed chat input
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

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs)

So `mock:terminal ...` and `mock:terminal-kill ...` are expected to fail in
Nori today even though they work in `acpx`. Terminal ACP behavior should be
validated with `acpx`, not treated as a `agent acp` interoperability failure
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
existing `mock:*` prompt text that the local `agent acp` mock agent already
understands.

The prompt pack intentionally excludes terminal commands because Nori's ACP
client still returns `method_not_found` for terminal methods. Keep using
`acpx` for:

- `mock:terminal ...`
- `mock:terminal-kill ...`

One more iteration detail: because the local Nori agents are launched through
`cargo run`, source changes are only picked up when Nori starts a fresh
conversation. After editing `agent-acp`, run `/new` before re-testing so Nori
respawns the agent process.
