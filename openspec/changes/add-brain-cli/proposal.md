# Proposal: add-brain-cli

## Why

The workspace needed a real user-facing binary rather than example-only entry
points. The old `cli-echo` and `cli-local` demos duplicated bootstrap logic,
used ad hoc provider setup, and did not reflect the runtime-first architecture
that now exists in `brain-acp`.

`brain-cli` is now the supported interactive/admin surface for local use, but
the active OpenSpec text still describes an older `BrainServer` / `BrainApi`
hosting model that is no longer the current design.

## What

This change defines the runtime-based CLI that exists today:

1. `brain-cli` is a real workspace binary at `crates/brain-cli/`.
2. The CLI boots an embedded `BrainRuntimeNative` and then works through
   `Arc<dyn BrainRuntime>`, matching the ACP backend dependency shape.
3. Project bootstrap uses `runtime.resolve_or_create_project(...)`.
4. Interactive chat uses `runtime.turn(...)`.
5. Session, message, and credential CRUD use `runtime.store()`.
6. Provider discovery is credential-driven and runtime-native:
   - API-key providers are discovered from stored credentials
   - OAuth providers are discovered from stored OAuth credentials when enabled
   - local providers are discovered from feature-gated local backends
   - `mock` remains the fallback when nothing else is available
7. The CLI uses the shared global `FileStore` rooted at `brain_home()`.
8. `brain-cli` provides the currently implemented command surface:
   - `brain` for interactive chat
   - `brain sessions list`
   - `brain sessions resume <id>`
   - `brain credentials add|login|list|remove`
   - `brain acp` as a stdio compatibility alias into `brain-acp`

This change does not define `brain serve` or `brain attach`. Those are deferred
until the remote-runtime/server path is designed around `BrainRuntime` rather
than the older `BrainApi` client boundary.

## Impact

- Modified capability: `brain-cli`
- Modified capability: `brain-core`
- Removed workspace members: `examples/cli-echo`, `examples/cli-local`
- Deferred from active workspace: `brain-server`, `examples/server`
