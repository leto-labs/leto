# Add Provider-Anthropic Crate

## Why

`provider-openai` gave the workspace a clean OpenAI-first protocol crate, but it
is still only one provider family. We need a second standalone wire client with
materially different request and streaming semantics before designing a richer
shared `Provider2` trait.

Anthropic's Messages API is a good counterexample because it is:

- message-history based rather than response-resource based
- SSE-first with a distinct event family
- multimodal
- tool-use capable without being OpenAI-shaped

## What Changes

- Add a new workspace crate: `provider-anthropic`
- Model Anthropic Messages request, response, and stream-event types in that
  crate
- Add a concrete client with non-streaming create and SSE stream support
- Add isolated unit tests and env-gated live smoke tests
- Add `@anthropic-ai/sdk` to `repocache` as a pinned local reference

## Impact

- Adds a new standalone protocol crate with no `brain-*` dependencies
- No `brain-types`, `brain-providers`, or runtime integration changes yet
- Defers `Provider2` until both standalone crates can be compared directly
