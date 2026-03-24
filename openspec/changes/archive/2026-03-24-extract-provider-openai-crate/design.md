# Design

## Core decision

Do not carry `Provider2` forward as a shared `brain` abstraction in this change.
The extracted crate should be OpenAI-first and expose a concrete client API,
because the current type model is explicitly shaped around the OpenAI Responses
API rather than a proven cross-provider contract.

## Boundary

`provider-openai` owns:

- request and response types for the Responses API
- streaming event types
- SSE stream handling
- WebSocket stream handling
- response parsing
- the concrete client used to call the OpenAI Responses endpoints

`brain` keeps:

- the legacy `Provider` trait
- the legacy `OpenAiProvider`
- preset catalogs and model metadata for the existing runtime/provider routing

## Testing

The new crate should rely on:

- unit tests for serialization and parser behavior
- env-gated live tests for non-streaming, SSE, WebSocket, and resource methods

It should not introduce a new generic mock provider. The old `MockProvider`
stays in `brain-providers` and remains focused on the legacy `Provider` trait.
