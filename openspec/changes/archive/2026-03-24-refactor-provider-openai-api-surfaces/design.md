# Design

## Core decision

Model OpenAI wire surfaces explicitly instead of keeping a flat client:

- `responses`
- `chat_completions`

Do not model legacy `/completions` in this change.

## Public shape

The crate exposes:

- `Client`
- `Client::responses()`
- `Client::chat_completions()`

Each surface client borrows the top-level client and uses shared config and the
shared `reqwest::Client`.

## Shared code

Shared logic stays minimal:

- auth header construction
- base URL helpers
- HTTP status/error handling
- common token-usage type

Parsers and schema types remain surface-specific unless they are actually
identical.

## References

- OpenAI Responses overview:
  <https://developers.openai.com/api/reference/responses/overview>
- OpenAI Chat Completions overview:
  <https://developers.openai.com/api/reference/chat-completions/overview>
- OpenAI Node SDK local structural reference:
  - `src/resources/responses/responses.ts`
  - `src/resources/chat/completions/completions.ts`
  - `src/client.ts`
