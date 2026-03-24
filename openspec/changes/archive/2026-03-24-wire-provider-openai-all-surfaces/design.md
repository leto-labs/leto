# Design: Wire provider-openai across all declared API surfaces

## Summary

This change makes the shared `OpenAiProvider` adapter honor
`Config::resolved_api_surface()` instead of assuming the Responses API.

The protocol-native client already supports:

- Responses
- Chat Completions

The shared adapter will now support both as well.

## Surface dispatch

`OpenAiProvider::stream()` will:

1. resolve the active API surface from `Config`
2. map the shared `provider::Request` into the corresponding wire request type
3. call the correct protocol-native client surface
4. translate that surface's stream into shared `provider::Event`s

If the requested or forced surface is unsupported, the adapter returns a clear
configuration/runtime error.

## Shared event translation

Responses remains the richer surface and keeps the existing translation.

Chat Completions gets a dedicated translation into the shared event model:

- visible text deltas become text blocks
- tool-call chunks become tool-call blocks with JSON deltas
- usage maps into shared `Usage`
- finish reasons map into shared `FinishReason`

The adapter preserves only the structure actually available from Chat
Completions. It must not fabricate richer semantics than the wire surface
provides.

## Capabilities

`OpenAiProvider::info()` becomes surface-aware.

Capabilities are computed from the resolved surface:

- forcing `ChatCompletions` narrows the advertised capability set to that
  surface
- forcing `Responses` advertises the richer Responses shape
- `Auto` reports capabilities for the surface that would actually be used

This keeps `ProviderInfo` honest for callers that rely on its capability
metadata.

## Smoke tests

Smoke coverage becomes matrix-based instead of single-path:

- iterate every built-in preset
- for each preset with credentials, iterate every supported API surface
- run wire-client smoke checks for each surface
- run shared-adapter smoke checks for each surface

Missing credentials and clear auth/quota/rate-limit errors still skip cleanly.
Declared preset/surface combinations that fail due to parser or compatibility
drift should fail the tests.
