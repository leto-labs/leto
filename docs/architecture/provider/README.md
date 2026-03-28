# Providers

The current provider story is split between the standalone shared `provider`
SDK and concrete `provider-*` crates such as `provider-openai` and
`provider-anthropic`. Legacy `brain-providers` code remains in the repo, but it
is no longer the primary naming for current-facing docs.

## Table Of Contents

| Topic | Document |
| --- | --- |
| Provider overview | [`README.md`](README.md) |
| OpenAI-compatible provider family | [`openai.md`](openai.md) |
| OAuth-backed OpenAI provider | [`openai_oauth.md`](openai_oauth.md) |
| Local `mistralrs` provider | [`mistralrs.md`](mistralrs.md) |
| Local `llama.cpp` provider | [`llamacpp.md`](llamacpp.md) |

## Provider Families

| Family | What it covers | Notes |
| --- | --- | --- |
| `mock` | Deterministic local echo/testing provider | Always available baseline |
| `provider-openai` | OpenAI-compatible HTTP providers and typed presets | Broadest external-provider path |
| `provider-openai` OAuth path | OAuth-backed OpenAI access built on the shared credential pool | Builds on stored or pooled OAuth credentials |
| `provider-mistralrs` | In-process local inference via `mistralrs` | Feature-gated |
| `provider-llamacpp` | In-process local inference via `llama.cpp` | Feature-gated |
| `pool` | CredentialPool and credential resolution/refresh | Shared auth substrate |
| `strategy` | `Fallback` and `StickyRoundRobin` selection policies | Used by pooled providers |

## What This Crate Actually Owns

| Concern | Current role |
| --- | --- |
| Provider implementations | yes |
| Provider metadata and model lists | yes |
| Preset catalogs for OpenAI-compatible vendors | yes, primarily through `provider-openai` |
| API-key and OAuth credential routing | yes |
| Local model provider configs | yes |

## Shared Runtime Contract

The provider boundary is intentionally small, but it is no longer just "send
text to an API."

| Shared type | Why it matters |
| --- | --- |
| `provider::Provider::stream(request)` | One entrypoint for all inference, including multimodal turns |
| streamed provider events | Streaming is the default contract, not an optional add-on |
| `provider::Message` and content blocks | Multimodality enters through the shared message model |
| `ModelInfo` | Model capability metadata travels alongside provider listings |

## Credential Path

```mermaid
flowchart LR
    Store[Credential store]
    Pool[CredentialPool]
    Strategy[Selection strategy]
    Provider[Concrete provider]
    HTTP[OpenAI-compatible API]

    Store --> Pool
    Strategy --> Pool
    Pool --> Provider
    Provider --> HTTP
```

## Provider Boundary And Loop Pressure

```mermaid
flowchart LR
    Loops[agent-loops]
    Runtime[agent-runtime request + event flow]
    Providers[provider / provider-* adapters]
    APIs[OpenAI-compatible APIs, Codex Responses, local runtimes]

    Loops --> Runtime
    Runtime --> Providers
    Providers --> APIs
```

## Architectural Reading

| Observation | Why it matters |
| --- | --- |
| The provider contract is message-first and stream-first | Loops and runtimes call one shared request/stream entrypoint and always consume a stream |
| OpenAI-compatible coverage is the broadest path | The repo is using that surface as the main external-provider compatibility layer |
| The trait is broader than one wire protocol | The same boundary also adapts Responses, Chat Completions, Anthropic Messages, and local providers |
| Multimodality entered through structured shared message/content blocks | The recent widening happened in shared message types, not by adding a second provider API |
| Loop pressure is what widened this seam recently | `terminus_kira` forced image-aware prompts and serializer changes across the provider layer |
| Pooling and selection strategy are reusable seams | Credential handling is not hardcoded into one provider instance |

## Related Reading

| Topic | Document |
| --- | --- |
| Cross-cutting provider-trait research | [`../../research/competitors/provider.md`](../../research/competitors/provider.md) |
| Loop overview and pressure points | [`../loops/README.md`](../loops/README.md) |
| Shared type definitions | [`../types.md`](../types.md) |
