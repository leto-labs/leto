# Neovim ACP Clients

## Summary

For a final keyboard-first UX pass against local `brain acp`, both
`agentic.nvim` and `codecompanion.nvim` are now worth keeping in `repocache`.
They are both real ACP clients, but they optimize for different things.

The practical recommendation for `brain` today is:

- use `codecompanion.nvim` as the primary Neovim ACP client when the goal is to
  exercise as much of `brain-acp`'s client-owned surface as possible
- use `agentic.nvim` as the secondary Neovim ACP client when the goal is the
  most ACP-centric shell with built-in session restore and simpler provider
  switching

Neither client currently gives a complete ACP terminal-operation UX. Keep
`acpx` as the fallback client when you need to validate `terminal/*` behavior.

## Why Both Matter

### `agentic.nvim`

`agentic.nvim` is architected as an ACP-first Neovim shell.

Strong points from source:

- dedicated ACP client and stdio transport modules under
  `lua/agentic/acp/`
- simple custom-provider configuration via `acp_providers`, where each provider
  is just `command`, optional `args`, optional `env`, and optional
  `default_mode`
- built-in session persistence and restore via local chat-history storage
- explicit provider switching, model switching, and mode switching in the UI
- permission queue UI and diff preview integrated into the chat flow

Important limitations for `brain`:

- the client advertises `fs.readTextFile = false`, `fs.writeTextFile = false`,
  and `terminal = false`
- incoming `fs/read_text_file` and `fs/write_text_file` notifications are
  explicitly ignored by the ACP client

That makes `agentic.nvim` excellent for checking multi-turn ACP chat UX, mode
changes, permissions, and restore behavior, but weak for validating the
file-read/file-write part of `brain-acp`'s external-client surface.

## `codecompanion.nvim`

`codecompanion.nvim` is a broader Neovim AI platform, but its current ACP
implementation is more complete for `brain`'s client-owned ACP features.

Strong points from source:

- ACP is now a first-class adapter family, separate from HTTP adapters
- built-in ACP docs describe session lifecycle, file operations, permissions,
  and dynamic ACP slash commands
- the ACP connection layer implements `session/new`, `session/load`,
  `fs/read_text_file`, `fs/write_text_file`, mode selection, model selection,
  and permission handling
- ACP permissions are surfaced with interactive approval plus diff preview
- the plugin supports command variants per ACP adapter, so agent launch can be
  customized

Important limitations for `brain`:

- terminal operations are explicitly unsupported and `terminal = false`
- agent plans are received but not rendered in the chat UI
- session management is documented as create/load/persist, but "No restore"
  remains the current limitation
- there is no obvious generic built-in `brain` adapter; using `brain acp`
  cleanly will likely require a small custom ACP adapter or extending an
  existing ACP preset

## Decision For `brain acp`

If the question is "which Neovim client should be the primary final UX check
for `brain acp` right now?", the answer is `codecompanion.nvim`.

Why:

- it exercises file read/write requests, which `brain-acp` already exposes and
  which `agentic.nvim` does not currently advertise
- it has explicit ACP permission UX with diff preview
- it supports session load plus dynamic ACP slash commands and mode/model flows
- it is closer to the broadest real client-capability coverage available in
  Neovim today

`agentic.nvim` remains valuable, but as a secondary view:

- it is the cleaner ACP-first shell
- it is easier to reason about as "Neovim as an ACP client"
- it has better built-in restore semantics for continuing a prior Neovim chat

So the split should be:

- primary manual UX client: `codecompanion.nvim`
- secondary ACP-shell comparison client: `agentic.nvim`
- terminal-operation fallback: `acpx`

## What This Means For Setup

For `brain`, the main challenge is not protocol support. It is client
configuration shape.

- `agentic.nvim` is simpler to point at a custom ACP server because
  `acp_providers` can directly name a command and args
- `codecompanion.nvim` has stronger ACP coverage, but `brain acp` likely needs
  a small custom ACP adapter entry rather than pretending to be one of the
  preset agents

That setup cost is acceptable because the final UX pass should optimize for
coverage of `brain-acp`, not minimal Neovim config.

## Source Highlights

- `agentic.nvim` README: `repocache/carlos-algms/agentic.nvim/README.md`
- `agentic.nvim` ACP transport/client:
  `repocache/carlos-algms/agentic.nvim/lua/agentic/acp/`
- `agentic.nvim` session restore:
  `repocache/carlos-algms/agentic.nvim/lua/agentic/session_restore.lua`
- `codecompanion.nvim` ACP docs:
  `repocache/olimorris/codecompanion.nvim/doc/agent-client-protocol.md`
- `codecompanion.nvim` ACP adapter config:
  `repocache/olimorris/codecompanion.nvim/doc/configuration/adapters-acp.md`
- `codecompanion.nvim` ACP implementation:
  `repocache/olimorris/codecompanion.nvim/lua/codecompanion/acp/`
