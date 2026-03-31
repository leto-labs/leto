# ACP Client Capability Matrix

Source-first comparison of ACP capability coverage relevant to `brain`.

This matrix is intentionally conservative. Each cell reflects one of:

- `Confirmed` — explicit source evidence was found
- `Partial` — some support is evident, but not the full surface
- `Not found in inspected path` — the inspected source path did not show a
  concrete implementation
- `Protocol-defined` — part of ACP itself rather than a client implementation

This repo was last updated from local source inspection on 2026-03-20.

## Matrix

| Capability | ACP protocol | `acpx` | Nori | Codex ACP | current `agent-acp` |
| --- | --- | --- | --- | --- | --- |
| `session/load` | Protocol-defined | Confirmed | Confirmed | Confirmed | Confirmed |
| model selection | Protocol-defined, unstable `session/set_model` or config replacement | Partial | Confirmed | Confirmed | Confirmed via `session/set_config_option` |
| session modes | Protocol-defined | Confirmed | Not found in inspected path | Confirmed | Not found in inspected path |
| session config options | Protocol-defined | Confirmed | Not found in inspected path | Confirmed | Confirmed |
| `session/set_config_option` | Protocol-defined | Confirmed | Not found in inspected path | Confirmed | Confirmed |
| filesystem client methods | Protocol-defined | Partial | Confirmed | Confirmed | Mock confirmed, real backend not yet client-owned |
| terminal client methods | Protocol-defined | Partial | Not found in inspected path | Confirmed | Mock confirmed, real backend not yet client-owned |
| permission requests | Protocol-defined | Partial | Confirmed | Confirmed | Mock confirmed |
| thought/reasoning chunks | Protocol-defined | Partial | Confirmed | Confirmed | Mock confirmed, real backend depends on runtime event mapping |

## Why These Verdicts

### ACP Protocol

ACP itself clearly defines:

- session modes
- session config options
- `session/set_config_option`
- client-owned filesystem and terminal methods

Primary references:

- <https://agentclientprotocol.com/protocol/session-modes>
- <https://agentclientprotocol.com/protocol/session-config-options>
- <https://agentclientprotocol.com/protocol/schema#session%2Fset-config-option>
- [session-modes.mdx](../../../repocache/agentclientprotocol/agent-client-protocol/docs/protocol/session-modes.mdx)

### `acpx`

The strongest concrete evidence found in the local source is that the `acpx`
runtime explicitly advertises control capabilities including:

- `session/set_mode`
- `session/set_config_option`
- `session/status`

and its runtime tests exercise those controls directly.

References:

- [`repocache/openclaw/openclaw/extensions/acpx/src/runtime.ts`](../../../repocache/openclaw/openclaw/extensions/acpx/src/runtime.ts)
- [`repocache/openclaw/openclaw/extensions/acpx/src/runtime.test.ts`](../../../repocache/openclaw/openclaw/extensions/acpx/src/runtime.test.ts)

The `Partial` verdicts on filesystem, terminal, permission, and thought chunks
reflect that this doc did not do a full bottom-up audit of every `acpx` prompt
event shape. `acpx` remains the baseline interop client, but this matrix should
not overstate more specific UI guarantees than were re-verified here.

### Nori

Confirmed in the inspected ACP path:

- `session/load`
- model-state-driven `session/set_model`
- filesystem client methods
- permission requests
- reasoning/thought rendering

Not found in the inspected ACP path:

- a generic session-config-options flow
- a `session/set_config_option` client path
- a clear external-agent session-mode picker flow

References:

- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/backend/session.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/public_api.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/public_api.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/pickers.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/chatwidget/pickers.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/tui/src/history_cell/mod.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/tui/src/history_cell/mod.rs)
- [`repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs`](../../../repocache/tilework-tech/nori-cli/codex-rs/acp/src/connection/client_delegate.rs)

### Codex ACP

Codex ACP is the clearest current reference for a rich ACP session-control
surface. The inspected source shows config options used for model and reasoning
effort, with update handling through `session/set_config_option`.

References:

- [`repocache/zed-industries/codex-acp/src/thread.rs`](../../../repocache/zed-industries/codex-acp/src/thread.rs)

### Current `agent-acp`

Current `agent-acp` is split:

- real backend: confirmed `session/load`, prompt flow, cancel, and session
  config options for model / thought level / loop
- mock backend: confirmed session modes, config options, permission flow, file
  flows, terminal probes, and thought chunks for ACP UX validation

The real backend does **not** yet expose the richer ACP mode/config-option
surface. The mock backend proves the protocol shape, but it is not the runtime
truth we want to preserve long term.

References:

- [`crates/agent-acp/src/adapter.rs`](../../crates/agent-acp/src/adapter.rs)
- [`crates/agent-acp/src/capabilities.rs`](../../crates/agent-acp/src/capabilities.rs)
- [`crates/agent-acp/src/mock/capabilities.rs`](../../crates/agent-acp/src/mock/capabilities.rs)

## Working Conclusion

For this repo, the cleanest current split is:

- `acpx` for baseline ACP compatibility and control-surface probing
- Nori as the strongest TUI base
- Codex ACP as the best implementation reference for config options

And the current product recommendation is:

- do **not** contort `agent-acp` into a client-specific bridge just because the
  current Nori ACP path is missing generic mode/config support
- if richer ACP-native session settings in the main TUI are the goal, extend or
  fork Nori instead
