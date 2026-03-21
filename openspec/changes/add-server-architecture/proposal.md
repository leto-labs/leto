# Proposal: add-server-architecture

## Why

The long-term direction is one client-facing interface, `BrainRuntime`,
regardless of whether a client is embedded in-process or talking to a remote
backend. The earlier `BrainServer` / `BrainApi` plan no longer matches that
direction:

- `brain-acp` already depends on `Arc<dyn BrainRuntime>`
- `brain-cli` now also depends on `Arc<dyn BrainRuntime>`
- `brain-server` is currently out of the active workspace

What remains valuable is the remote-runtime goal itself: a future server should
let clients use the same runtime concepts and high-level behaviors whether the
implementation is embedded (`BrainRuntimeNative`) or remote.

## What Changes

This change is now deferred future work for a remote-runtime/server path.

The intended architecture is:

1. `BrainRuntimeNative` remains the embedded implementation used by local
   surfaces such as ACP and the CLI.
2. A future remote implementation, `BrainRuntimeRemote`, will also
   implement `BrainRuntime`.
3. That remote runtime will provide proxy `Store` and registry objects so the
   client-facing interface remains the same across embedded and remote modes.
4. A future `brain-server` will host `BrainRuntimeNative` and expose the
   transport needed by the remote runtime client.
5. The remote/server work is no longer the dependency boundary for current CLI
   or ACP implementation.

This change should therefore document the future remote-runtime architecture,
not claim that `BrainServer` / `BrainApi` is the current primary client path.
The existing out-of-workspace `brain-server` crate is legacy code that should be
treated as a historical implementation to replace rather than as the target
design.

## Impact

- Modified capability: `brain-server`
- Modified capability: `brain-core`
- Modified capability: `brain-transports`
- Deferred from active workspace: `brain-server`, `examples/server`
- Architectural cleanup target: replace `BrainApi`-centric server/client
  concepts with `BrainRuntimeRemote` parity over `BrainRuntime`
