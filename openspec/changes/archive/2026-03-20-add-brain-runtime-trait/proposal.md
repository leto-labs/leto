# Proposal: add-brain-runtime-trait

## Why

`brain-types` defines the low-level domain contracts for providers, tools,
stores, loops, and transports, but it does not yet define the shared runtime
boundary that app surfaces can depend on.

That gap forces crates such as `brain-acp` and `brain-server` to integrate with
`brain-core::Brain` as a concrete type instead of depending on a stable
runtime-facing trait.

## What

- add a new `BrainRuntime` trait to `brain-types`
- add a dedicated `registry.rs` module to `brain-types`
- add a generic `Registry` trait to that module
- add `RegistryProvider`, `RegistryTool`, and `RegistryLoop` specializations
- add a reusable `RegistryHashMap` implementation and provider/tool/loop aliases
- keep the trait focused on runtime/session/turn behavior
- keep `store()` available for raw CRUD
- add store-emitted project/session lifecycle events so direct store mutations remain observable
- expose provider/tool/loop management through registry accessors on `BrainRuntime`
- keep `BrainRuntime` limited to runtime-owned behavior rather than mirroring store CRUD
- add a live shared runtime bus for protocol-neutral lifecycle and duplexed turn events
- expose model inspection and model selection on the runtime because those depend on provider registry state
- keep registry listing focused on returning the live named items themselves,
  rather than derived metadata
- add the first concrete implementation as `BrainRuntimeNative` in `brain-core`
- add loop selection to shared agent/session config so the runtime can select
  among multiple registered loops
- migrate `brain-cli` to the runtime trait so CLI and ACP share the same
  embedded runtime boundary
- defer `brain-server` from the active workspace while the runtime-facing
  client shape settles

## Impact

- Modified capability: `brain-types`
- Modified capability: `brain-stores`
- Modified capability: `brain-core`
- Modified capability: `brain-acp`
- Modified capability: `brain-cli`
