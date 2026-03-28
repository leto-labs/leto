# OpenAI Provider Family

This page covers the OpenAI-compatible provider family in `provider-openai`:
the HTTP client, shared-provider adapter, typed preset catalog, API-surface
selection, and serializer behavior.

## Main Pieces

| Component | Role |
| --- | --- |
| `OpenAiProvider` | Concrete provider implementation for OpenAI-compatible HTTP endpoints |
| `OpenAiConfig` | Runtime config including base URL, model, provider name, and API surface mode |
| `OpenAiConfigPreset` | Preset catalog for multiple vendors that expose an OpenAI-like API |
| `OpenAiApiMode` / `OpenAiApiSurface` | Controls Responses vs Chat Completions resolution |

## Important Behaviors

| Concern | Current behavior |
| --- | --- |
| Surface selection | `auto` prefers Responses when the preset supports it, otherwise Chat Completions |
| Streaming | SSE parsers exist for both supported API surface families |
| Tool calls | Native provider tool calls flow back into the shared `ChatChunk` surface |
| Multimodal input | Message parts with text and image URLs can be serialized for OpenAI-compatible requests |

## Why It Is Central

The OpenAI-compatible path is the broadest provider surface in the repo. It is
used not only for OpenAI itself but also for many preset-driven compatible
providers, which makes it the main external inference abstraction in practice.
