## Overview

The new v2 stack uses a narrow ownership split:

- `provider` owns canonical transcript/message schemas
- `atif` owns canonical trajectory/export schemas
- `agent-runtime` owns live in-memory execution and control flow
- `agent-store` owns durable records and persistence traits
- `agent-core` owns multi-session orchestration and default stack assembly

This deliberately avoids rebuilding `brain-types` as another shared dumping
ground.

## Store Model

`agent-store` persists durable records only:

- `Project`
- `Session`
- `StoredMessage`
- `CredentialEntry`
- `atif::Trajectory`

`StoredMessage` wraps `provider::Message` plus persistence metadata. Persisted
`Session` remains distinct from `agent-runtime::SessionState`, which contains
live queues, waits, PTY state, and other runtime-only data.

## Core Model

`agent-core` is the new app-facing runtime SDK. It:

- holds the store
- registers providers, loops, and tools
- resolves effective session runtime config
- constructs a per-turn `SessionEngine`
- forwards runtime events
- persists the canonical transcript back to the store

The initial implementation uses one-shot runtime engines per turn and adds a
small `SessionCommand::Shutdown` command so the outer core layer can terminate
those engines cleanly after the turn completes or is cancelled.

## Tool Model

`agent-tools` is the concrete v2 tool crate for the new stack. It:

- keeps typed Rust request/response contracts per tool
- keeps swappable driver traits behind those contracts
- implements the runtime-facing `ToolExecutor` boundary through an erased
  registry
- generates tool schemas directly from typed request/response types via
  `schemars`

The v1 schema policy is intentionally simple:

- use direct `schemars` output for tool input and output schemas
- use Rust doc comments as the default source of field/object descriptions
- defer any provider-specific schema normalization until real compatibility
  problems appear

`agent-core` now installs the native `agent-tools` executor by default instead
of bridging legacy `brain-types::Tool` implementations.
