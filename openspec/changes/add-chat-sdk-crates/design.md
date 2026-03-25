# Design: add-chat-sdk-crates

## Decision: Mirror The Provider Crate Family

The chat SDK should follow the same shape as the standalone provider work:

- `chat` owns the shared abstraction
- `chat-*` crates own provider-specific config and normalization

This keeps the reusable SDK boundary small and prevents top-level application
concerns from leaking into transport/provider crates.

## Decision: Shared Chat Crate Is Not Agent-Aware

The shared `chat` crate models chat-provider concepts only:

- adapter identity
- inbound chat events
- outbound message operations
- attachments
- capability discovery
- health checks

It does not model:

- sessions
- stores
- tools
- loops
- approvals
- agents
- runtime/server policy

Future high-level layers may compose these primitives, but the SDK remains
generic.

## Decision: Normalize By Capability, Not By Forced Uniformity

Telegram, Slack, and Teams do not expose identical features. The shared crate
therefore uses explicit capability discovery for optional features such as:

- message edits
- threaded replies
- typing indicators
- reactions
- attachments
- webhook mode
- polling mode

Concrete crates may support different subsets without inventing fake behavior.

## Decision: Keep The Initial Surface Small

The first implementation favors a compact public surface:

- one adapter trait
- one normalized event stream type
- one normalized inbound event enum
- plain data types for conversations, messages, participants, attachments, and
  delivery receipts

That is enough for future consumers to build on without prematurely baking in
application-specific routing or workflow semantics.
