# Extract Provider-OpenAI Crate

## Why

The OpenAI Responses v2 work is currently split awkwardly across `brain-types`
and `brain-providers`. That makes the ownership boundary confusing: an
OpenAI-specific request/event model sits in the shared `brain` type crate, while
the concrete parser and transport code lives elsewhere.

This change extracts that stack into a standalone `provider-openai` crate with
no `brain-*` dependencies. That keeps the OpenAI Responses client isolated,
makes its tests independent, and leaves `brain` free to design a future generic
provider trait later instead of baking that design into `brain-types` now.

## What Changes

- Add a new workspace crate: `provider-openai`
- Move the full Responses v2 request/response/event model into that crate
- Move the concrete OpenAI Responses client, parser, SSE handling, and
  WebSocket handling into that crate
- Remove `Provider2` and `Response*` ownership from `brain-types`
- Remove the v2 Responses implementation and tests from `brain-providers`
- Keep the legacy `OpenAiProvider` and legacy `Provider` path in
  `brain-providers` unchanged

## Impact

- Breaking for the experimental v2 surface currently exposed from `brain-types`
- No intended behavior change for the legacy `brain` runtime or CLI paths
- Adds a new standalone crate with its own isolated unit and integration tests
