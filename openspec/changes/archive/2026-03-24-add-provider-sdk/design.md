# Design

## Core decision

Create a standalone `provider` crate rather than extending `brain-types`.

The new crate owns the shared inference contract and treats `brain` as one
future consumer rather than the defining runtime. `provider-openai` and
`provider-anthropic` remain the owners of their native wire protocols and add
adapters into the shared contract.

## Shared SDK shape

The shared crate exposes:

- `Provider`
- `ProviderInfo`
- `ProviderCapabilities`
- `Request`
- `Message`, `MessageRole`, and shared content blocks
- shared tool definitions and tool-choice options
- a stream-first `Event` model
- `Usage`, `FinishReason`, and shared errors
- `MockProvider`

The trait stays stream-first and inference-focused. It does not model agent-loop
operations such as steering, approvals, or mid-turn user injection.

## Event model

The shared event model is provider-neutral and block-oriented:

- response start
- block start
- block delta
- block stop
- usage
- completed

The block layer is the common denominator across:

- OpenAI Responses output text, tool-call argument deltas, reasoning summaries,
  and refusals
- Anthropic Messages content-block start/delta/stop events
- local or mock providers that only emit a single text block

The shared crate does not re-export raw OpenAI or Anthropic event enums.

## Adapter strategy

`provider-openai` adds an `OpenAiProvider` that:

- maps shared requests into Responses API requests
- streams OpenAI Responses events
- translates them into shared provider events

`provider-anthropic` adds an `AnthropicProvider` that:

- maps shared requests into Messages API requests
- streams Anthropic SSE events
- translates them into shared provider events

Adapters are allowed to reject unsupported shared request combinations with a
shared provider error instead of silently degrading.

## Migration

This change lands in parallel with the legacy `brain-types::Provider` and
`brain-providers` stack.

It does not refactor `brain` to use the new shared crate yet. The immediate goal
is to establish a stable generic provider SDK surface plus two real plugin
implementations and a mock implementation.
