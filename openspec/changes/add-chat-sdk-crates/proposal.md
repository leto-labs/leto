# Add Chat SDK Crates

## Why

The workspace now has a clean split between a shared `provider` crate and
provider-specific `provider-*` crates, but it has no equivalent SDK boundary
for external chat surfaces.

We want the same style of decomposition for messaging providers:

- one shared Rust-native `chat` crate
- separate `chat-*` implementation crates
- no `brain-*` or agent-runtime coupling at the SDK layer

This change intentionally stays below `BrainRuntime` and `BrainServer`. Those
high-level layers may later consume the chat SDK through simple traits, events,
and streams, but they are not part of this implementation.

## What Changes

- Add a standalone `chat` crate with shared adapter traits, normalized event and
  message types, capability discovery, and error types
- Add `chat-telegram`, `chat-slack`, and `chat-teams` crates that implement the
  shared adapter trait
- Add normalization-focused tests for the shared and concrete crates
- Add a new OpenSpec change with capabilities for `chat`, `chat-telegram`,
  `chat-slack`, and `chat-teams`

## Impact

- Adds four new workspace crates with no `brain-*` dependencies
- Does not modify `BrainRuntime`, `BrainServer`, or `brain-transports`
- Creates a reusable SDK layer for future runtime/server integration work
