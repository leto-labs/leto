# Forge ACP Coverage

## Summary

`forge` is the strongest pure terminal ACP client we have inspected so far.
Unlike `acpx`, it is a real terminal UI built on OpenTUI/Solid, with a
dedicated ACP client layer, session history, plan rendering, model/mode
controls, and ACP-specific tests.

The main conclusion for `brain` is split:

- `forge` is a serious ACP-native TUI reference and worth keeping in
  `repocache`
- `forge` is not yet a clean immediate client for local `brain acp`, because
  its actual agent launch flow is registry-driven rather than a documented raw
  `--agent "<command>"` path like `acpx`

So the current recommendation is:

- keep `acpx` as the most reliable custom-agent ACP client
- keep `forge` as the strongest ACP TUI candidate and implementation reference
- revisit `forge` as a live `brain acp` client only if it gains a raw custom
  launcher or if `brain` is wrapped/published in a way Forge can resolve as an
  agent

## What Forge Covers Well

From source, Forge already has meaningful ACP product coverage:

- real TUI shell under `packages/forge/src/cli/cmd/tui/`
- dedicated ACP client/orchestrator under `packages/forge/src/acp/`
- prompt lifecycle: initialize, `session/new`, prompt, cancel, continue/session
  reuse
- permission requests via ACP `requestPermission`
- session mode discovery and switching
- session model discovery and switching
- plan updates, including explicit translation and sidebar rendering
- `available_commands_update` handling for agent command discovery
- ACP-focused tests under `packages/forge/test/acp/`

That makes Forge qualitatively different from editor-hosted ACP shells. It is a
true terminal-first ACP product, not just a plugin or adapter.

## Gaps And Risks For `brain acp`

### 1. Agent launch is registry-driven

Forge's user-facing model is:

- `forge <agent> ...`
- `forge agents`
- `forge <agent> install`

The resolved agent comes from Forge's registry layer in
`packages/forge/src/acp/agents.ts`, and agent selection in
`packages/forge/src/cli/cmd/session-init.ts` resolves against that registry.

Forge does have a `--agent` flag, but in current source it means:

- pick a Forge-known agent, optionally with model/mode overrides
- not run an arbitrary ACP subprocess command

The current flag shape is things like:

- `--agent claude`
- `--agent "name=claude model=opus mode=bypassPermissions"`

That is materially different from `acpx --agent "cargo run -q -p brain-cli -- acp"`.

### 2. File read/write is currently disabled as a client capability

Forge's ACP client wiring currently advertises:

- `fs.readTextFile = false`
- `fs.writeTextFile = false`

in both the connection path and orchestrator path.

That means Forge is not presently positioned as a high-coverage ACP client for
`brain-acp`'s `fs/*` client-owned surface, even though it is strong elsewhere.

### 3. No evidence of general `terminal/*` client support

The current ACP/TUI code clearly handles:

- permission requests
- terminal-auth metadata for login instructions
- normal terminal rendering concerns

But there is no comparable evidence in the inspected ACP layer that Forge is a
general client for ACP `terminal/*` methods.

So for `brain`'s external-client validation, Forge currently looks strongest on
multi-turn UX, plans, and mode/model control, but weak on the more advanced
client-owned file/terminal surface.

## ACP Test Coverage In Forge

Forge already tests ACP as a first-class subsystem. The test tree includes:

- basic flow integration tests
- MCP integration tests
- mode management tests
- model management tests
- plan lifecycle tests
- translator and tool/debug tests
- subprocess/orchestrator coverage

That gives confidence that ACP is central to Forge's architecture, not
incidental.

## Recommendation For `brain`

For `brain` today:

- use `forge` as a source reference for a serious ACP TUI
- keep it in `repocache` for continued study of terminal UX, session views,
  plan rendering, and ACP orchestration
- do not treat it as the primary immediate manual client for local `brain acp`
  unless we prove a custom agent launch path

Practical ranking after this analysis:

1. `acpx` for custom local `brain acp` interoperability
2. `forge` for the best ACP-native terminal UX reference and most promising
   future TUI client
3. Neovim clients for editor-hosted UX checks

## Source Highlights

- Forge README: `repocache/forge-agents/forge/README.md`
- Forge architecture: `repocache/forge-agents/forge/ARCHITECTURE.md`
- ACP agent registry wiring: `repocache/forge-agents/forge/packages/forge/src/acp/agents.ts`
- ACP client: `repocache/forge-agents/forge/packages/forge/src/acp/client.ts`
- ACP orchestrator: `repocache/forge-agents/forge/packages/forge/src/acp/orchestrator.ts`
- CLI agent resolution: `repocache/forge-agents/forge/packages/forge/src/cli/cmd/session-init.ts`
- CLI agent run path: `repocache/forge-agents/forge/packages/forge/src/cli/cmd/run.ts`
- ACP test suite: `repocache/forge-agents/forge/packages/forge/test/acp/`
