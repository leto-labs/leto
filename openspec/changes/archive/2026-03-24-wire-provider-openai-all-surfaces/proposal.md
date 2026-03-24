# Change: Wire provider-openai across all declared API surfaces

## Why

`provider-openai::Config` and its generated presets already declare which
OpenAI-compatible API surfaces each provider supports, but the shared
`OpenAiProvider` adapter still routes all inference through the Responses API.
This leaves a gap:

- presets can declare chat-completions-only support
- the wire client can already use both surfaces
- the shared adapter ignores that metadata and always uses Responses

We want the shared adapter to be fully wired across the surfaces each preset
declares, and we want smoke coverage for every supported preset/surface
combination so compatibility is proven rather than assumed.

## What Changes

- Make `OpenAiProvider` dispatch by resolved API surface
- Add shared-event translation for Chat Completions streaming
- Make shared capability metadata reflect the active resolved surface
- Expand smoke coverage to every supported preset/surface combination for both
  the wire client and the shared provider adapter

## Impact

- Affected spec: `provider-openai`
- Affected crate: `provider-openai`
