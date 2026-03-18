# Codex ACP Adapter

## Summary

`zed-industries/codex-acp` is one of the strongest ACP examples in the current
ecosystem because it adapts a serious coding agent into an ACP-compatible
surface while still exposing meaningful functionality:

- auth methods
- tool calls and permission requests
- client MCP servers
- slash commands
- mode/model/config controls
- terminal-aware tool streaming

This is strong evidence that ACP can front a real coding-agent runtime rather
than only a toy chat loop.

## Positioning

The README is explicit: this is "an ACP adapter around the Codex CLI." That
word matters. It is not pretending ACP is the whole runtime. It is wrapping an
existing runtime into a standard client surface.

That is likely the right mental model for `brain` too, at least initially.

## Confirmed Capabilities

From the README and code:

- auth methods include ChatGPT login, `CODEX_API_KEY`, and `OPENAI_API_KEY`
- tool calls are surfaced with permission requests
- client MCP servers are supported
- slash commands include `/review`, `/review-branch`, `/review-commit`,
  `/init`, `/compact`, `/logout`, and custom prompts
- session mode and model changes are implemented
- client filesystem methods are used when the client advertises them
- terminal output is streamed through tool call updates when the client can
  display it

This is not a minimal ACP shell. It is deliberately using ACP's richer client
surface.

## Design Patterns Worth Stealing

### 1. Adapter, not rewrite

The ACP layer wraps the existing Codex runtime. That keeps the runtime logic in
one place and avoids building a separate "ACP-only agent."

### 2. Use client capabilities when present

The code checks client FS capability flags before calling ACP file methods and
has explicit terminal-output support. That is the right model for `brain`:

- prefer editor-owned capabilities when present
- degrade gracefully when they are absent

### 3. Preserve real agent controls

Codex ACP does not flatten everything into one text channel. It preserves auth,
modes, models, config options, slash commands, and review workflows where it
can. That is a good reminder that ACP should expose `brain`'s real runtime
surface, not just "chat."

## What It Implies For `brain`

- `brain` should implement ACP as a surface over the main runtime, not as a
  second runtime.
- `brain` should expect rich ACP clients to want mode, model, config, review,
  and approval semantics.
- `brain` should use ACP filesystem and terminal methods when clients offer
  them, because serious adapters already do.

## Key Sources

- README: [`README.md`](../../repocache/zed-industries/codex-acp/README.md)
- main agent implementation: [`src/codex_agent.rs`](../../repocache/zed-industries/codex-acp/src/codex_agent.rs)
- terminal-aware thread logic: [`src/thread.rs`](../../repocache/zed-industries/codex-acp/src/thread.rs)
- client FS bridge: [`src/local_spawner.rs`](../../repocache/zed-industries/codex-acp/src/local_spawner.rs)
- npm package metadata: <https://www.npmjs.com/package/@zed-industries/codex-acp>
