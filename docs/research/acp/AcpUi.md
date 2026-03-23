# ACP UI

Source-first notes on `acp-ui` as a standalone ACP client relevant to
`brain acp`.

This repo was inspected from the local repocache clone on 2026-03-18.

## Short Answer

`acp-ui` is the cleanest standalone custom-agent launcher we have found so far.

If the question is "what standalone UI is most likely to talk to a custom local
`brain acp` command right now with minimal ceremony?", `acp-ui` is the best
answer so far.

The tradeoff is that it is a desktop app rather than a terminal TUI, and its
current ACP client capability surface is narrower than Nori's.

## What It Is

`acp-ui` is a Tauri/Vue desktop ACP client with an explicit agent launcher
configuration file.

Relevant paths:

- [`repocache/formulahendry/acp-ui/src/lib/acp-bridge.ts`](../../../repocache/formulahendry/acp-ui/src/lib/acp-bridge.ts)
- [`repocache/formulahendry/acp-ui/src/stores/session.ts`](../../../repocache/formulahendry/acp-ui/src/stores/session.ts)
- [`repocache/formulahendry/acp-ui/src-tauri/src/agent.rs`](../../../repocache/formulahendry/acp-ui/src-tauri/src/agent.rs)
- [`repocache/formulahendry/acp-ui/src-tauri/src/config.rs`](../../../repocache/formulahendry/acp-ui/src-tauri/src/config.rs)
- [`repocache/formulahendry/acp-ui/README.md`](../../../repocache/formulahendry/acp-ui/README.md)

## Custom ACP Launch

This is the cleanest custom-agent launch model of any standalone UI client we
have inspected so far.

`acp-ui` stores agent configs in `agents.json`, and each agent is just:

- `command`
- `args`
- `env`

The README documents that directly, and the Tauri backend spawns the configured
subprocess with stdout and stdin wired to the ACP bridge:

- [`repocache/formulahendry/acp-ui/README.md`](../../../repocache/formulahendry/acp-ui/README.md)
- [`repocache/formulahendry/acp-ui/src-tauri/src/agent.rs`](../../../repocache/formulahendry/acp-ui/src-tauri/src/agent.rs)
- [`repocache/formulahendry/acp-ui/src-tauri/src/config.rs`](../../../repocache/formulahendry/acp-ui/src-tauri/src/config.rs)

For `brain`, the launch path would be straightforward:

```json
{
  "agents": {
    "brain": {
      "command": "cargo",
      "args": ["run", "-q", "-p", "brain-cli", "--", "acp"],
      "env": {}
    }
  }
}
```

This is still an inference from the generic config format and spawn code, but
it is a much smaller inference than with most other clients.

## ACP Coverage

`acp-ui` has solid support for the core custom-client surfaces that matter for
`brain acp`, but its current ACP coverage is narrower than Nori's.

What is clearly present:

- initialize
- authenticate
- `session/new`
- `session/load`
- prompt
- cancel
- `session/set_mode`
- unstable `session/set_model`
- `fs/read_text_file`
- `fs/write_text_file`
- `session/request_permission`
- tool call rendering
- thought rendering
- available slash commands
- traffic monitor

Evidence:

- [`repocache/formulahendry/acp-ui/src/lib/acp-bridge.ts`](../../../repocache/formulahendry/acp-ui/src/lib/acp-bridge.ts)
- [`repocache/formulahendry/acp-ui/src/stores/session.ts`](../../../repocache/formulahendry/acp-ui/src/stores/session.ts)

Important limitations visible in source:

- `clientCapabilities` currently advertise only `fs.readTextFile` and `fs.writeTextFile`
- there is no terminal client-method handling in the ACP bridge
- there is no explicit plan update rendering path in the session store

That means `acp-ui` looks good for:

- chat
- multi-turn sessions
- file read/write
- permission prompts
- slash commands
- traffic inspection

But weaker for:

- terminal round-trips
- plan UX
- full parity with richer ACP desktop and editor shells

## Fit For `brain`

`acp-ui` is the strongest current standalone custom-agent UI candidate for
`brain acp` if "easy to configure and inspect" matters more than "must be a TUI".

Why:

- the custom launcher model is explicit and minimal
- traffic monitor is useful for ACP debugging
- file read/write and permission flows are implemented directly
- session load and resume are present

Risk:

- it is a desktop app, not a terminal TUI
- current ACP client capability coverage is intentionally smaller than Nori's
