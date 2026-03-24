# Design

## Core decision

Keep `provider-anthropic` protocol-native and Messages-focused. This change does
not attempt to normalize Anthropic into `brain-types`, and it does not add
`Provider2`.

## Public shape

The crate exposes:

- `Client`
- `Client::messages()`
- `Config`
- `Error`
- `messages::*` request, response, and stream types

The Messages surface supports:

- non-streaming create
- SSE streaming create

## Type scope

The request and response model focuses on the shared, high-value Messages
surface:

- text and image content
- tool definitions and tool-use/tool-result blocks
- thinking and redacted-thinking blocks
- stop reasons and usage
- SSE event families for message/content start, delta, and stop

The crate does not try to model every Anthropic beta surface or server tool in
v1. Unknown blocks and events are preserved as raw JSON when needed.

## Testing

The crate should include:

- unit tests for request serialization
- parser tests for non-streaming messages and SSE events
- env-gated live smoke tests using `ANTHROPIC_API_KEY`

Library code remains config-driven and must not read environment variables
outside tests.
