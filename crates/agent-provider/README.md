# Agent Provider Ecosystem

This folder groups the standalone crates that define and implement the provider
layer for an agent runtime.

The purpose of this layout is to keep provider code organized as one coherent
ecosystem: a shared provider SDK plus concrete provider implementations that
depend on it.

## Architecture

The provider ecosystem is split into two layers:

- `provider/`
  - the shared SDK and provider-facing contract
  - owns common request, event, capability, model, and credential abstractions
- `provider-*`
  - standalone provider implementation crates
  - adapt specific remote or local inference backends to the shared provider
    contract

This lets hosts target one stable provider interface while swapping concrete
backends independently.

## Dependency Direction

The intended dependency flow is:

- implementation crates depend on `provider/`
- runtimes or application surfaces depend on `provider/` plus the concrete
  `provider-*` crates they want to install
- `provider/` does not depend on any specific provider implementation

That separation keeps the base provider contract reusable and prevents one
backend's requirements from leaking into the shared SDK.

## Folder Layout

- `provider/` — shared provider SDK
- `provider-openai/` — OpenAI-compatible provider implementation
- `provider-anthropic/` — Anthropic provider implementation
- `provider-mistralrs/` — local `mistral.rs` provider implementation
- `provider-llamacpp/` — local `llama.cpp` provider implementation

Additional provider crates should follow the same pattern:

- depend on `provider/`
- implement one concrete backend family
- keep backend-specific transport, auth, and request translation logic inside
  the implementation crate rather than pushing it into the shared SDK
