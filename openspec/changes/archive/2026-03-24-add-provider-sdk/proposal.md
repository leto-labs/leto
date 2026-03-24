# Add Shared Provider SDK Crate

## Why

`provider-openai` and `provider-anthropic` now give the workspace two
protocol-native wire crates, but there is still no shared provider SDK surface
that can sit above them without inheriting `brain` runtime assumptions.

We want the next abstraction to behave more like a generic AI SDK than an
agent-runtime trait. That means the shared provider contract should live outside
`brain-types`, own its own request and event model, and allow OpenAI and
Anthropic to plug into it as standalone provider implementations.

## What Changes

- Add a new workspace crate: `provider`
- Move the new shared provider trait, request model, event model, capabilities,
  and mock implementation into that crate
- Add shared-trait implementations to `provider-openai` and
  `provider-anthropic`
- Keep the existing `brain-types::Provider` and `brain-providers` stack in
  place for now

## Impact

- Adds a reusable provider SDK substrate with no `brain-*` dependencies
- Keeps protocol crates protocol-native while making them usable as provider
  plugins
- Defers `brain` migration to a separate follow-up change
