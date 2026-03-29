# Add agent-acp

## Why

The repository is migrating its application-facing surfaces from legacy
`brain-*` crates to the refactored `agent-*` stack.

`brain-acp` is still coupled to the older `BrainRuntime`-oriented composition
path. That coupling keeps ACP integration on the legacy runtime boundary even
as the new stack standardizes on `AgentCore` as the shared consumer surface for
embedded and remote callers.

We want a dedicated `agent-acp` crate that replaces `brain-acp` as the
canonical ACP adapter and wraps `AgentCore` directly instead of `BrainRuntime`.

## What Changes

- add a new `agent-acp` capability for the refactored ACP crate
- define `agent-acp` as the canonical ACP adapter for the `agent-*` stack
- require the ACP backend to depend on `AgentCore` rather than legacy
  `BrainRuntime`
- preserve ACP session lifecycle, prompt execution, history replay, and session
  configuration behavior on top of `AgentCore`

## Impact

- adds a new workspace-facing ACP crate in the refactored stack
- establishes the next migration step away from `brain-acp`
- gives future ACP integrations a stable `AgentCore`-based boundary shared with
  the rest of the `agent-*` architecture
