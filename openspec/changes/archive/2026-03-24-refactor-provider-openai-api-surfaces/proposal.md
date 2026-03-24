# Refactor Provider-OpenAI API Surfaces

## Why

`provider-openai` currently behaves like a Responses client with a flat module
layout. That makes the crate awkward to extend now that OpenAI-compatible
providers may support:

- `responses`
- `chat/completions`
- sometimes both

We want `provider-openai` to remain a standalone wire client, but its structure
should reflect those API surfaces directly.

## What Changes

- Refactor `provider-openai` into explicit `responses` and `chat_completions`
  modules
- Add a top-level client façade that exposes both surfaces
- Keep shared code narrow and transport-focused
- Add first-class Chat Completions request/response/stream support
- Preserve existing Responses support and tests

## Impact

- Breaking for the experimental `provider-openai` public API
- No intended `brain-*` trait or runtime changes
- Establishes a clearer surface boundary for future provider capability routing
