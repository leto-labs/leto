## Why

The new `agent-runtime` stack has a strong per-session execution engine, but it
still lacks the store-backed application layer that legacy `BrainRuntime`
provided. Right now there is no v2 boundary that:

- persists projects, sessions, messages, credentials, and trajectories
- assembles providers, loops, and tools into a reusable app-facing SDK
- reuses canonical `provider` and `atif` schema crates instead of duplicating
  transcript and trajectory types

Without this layer, every future CLI, ACP adapter, or server surface will have
to reassemble the stack itself.

## What Changes

- add a new `agent-store` crate with clean-room store traits and concrete
  in-memory / file-backed implementations
- reuse `provider::Message` for persisted transcript content and `atif`
  trajectory types for trajectory persistence
- add a new `agent-core` crate that assembles store + providers + tools + loops
  around `agent-runtime`
- provide a default local builder with:
  - native `agent-tools`
  - `SimpleLoop`
  - credential-based OpenAI-compatible provider discovery
- add a new `agent-tools` crate with typed request/response contracts, direct
  `schemars`-generated schemas, and native local tool drivers
- extend shared tool definitions with optional `output_schema`
- add core turn orchestration that loads stored transcript state into
  `SessionEngine`, streams runtime events, and persists the final transcript back
  into the store

## Impact

- adds three new v2 crates: `agent-store`, `agent-core`, and `agent-tools`
- extends `agent-runtime` with a small shutdown command needed for clean
  core-managed turn lifecycle
- creates the new app-facing composition root above `agent-runtime` without
  reintroducing a giant catch-all shared types crate
