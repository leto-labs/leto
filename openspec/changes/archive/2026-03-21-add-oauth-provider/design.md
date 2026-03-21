# Design: add-oauth-provider

## Module Layout

Three new modules / files inside `brain-providers/src/`, plus additions to
`brain-types`:

```
brain-types/src/
├── credential.rs        # ProviderCredential enum + OAuthCredentials struct
├── model.rs             # ModelInfo, ModelCost, ModelLimit structs
└── traits.rs            # CredentialStore trait (added to Store supertrait)

brain-providers/src/
├── oauth/                  # Shared OAuth plumbing (provider-agnostic)
│   ├── mod.rs              # Re-exports from brain_types
│   ├── pkce.rs             # PKCE verifier + S256 challenge
│   ├── browser_flow.rs     # Auth code + PKCE via localhost callback
│   ├── device_flow.rs      # OpenAI proprietary device code polling flow
│   ├── refresh.rs          # Stateless token refresh
│   └── jwt.rs              # JWT claim extraction (no sig verification)
├── openai_oauth/           # OpenAI OAuth provider
│   ├── mod.rs
│   ├── preset.rs           # OpenAiOAuthPreset (endpoints, client ID, models)
│   └── provider.rs         # OpenAiOAuthProvider impl Provider
├── openai_sse.rs           # Shared SSE parsing for standard Chat Completions API
└── codex_sse.rs            # Responses API request/response types + SSE parsing
```

Both `oauth/` and `openai_oauth/` modules are gated behind
`feature = "openai-oauth"` (off by default). The `openai_sse` module is
available when either `openai` or `openai-oauth` is enabled. The `codex_sse`
module is only available under `openai-oauth`.

## Credential Architecture

### ProviderCredential (brain-types)

Rather than coupling OAuth tokens to a provider-specific store, credentials are
modelled as a sum type in `brain-types`:

```rust
enum ProviderCredential {
    ApiKey { api_key: String },
    OAuth(OAuthCredentials),
}
```

This keeps credential storage uniform — the same store handles API keys and
OAuth tokens. `OAuthCredentials` holds `access_token`, `refresh_token`,
`expires_at`, `client_id`, `token_endpoint`, and `account_id`.

### CredentialStore (brain-types trait)

A `CredentialStore` trait sits alongside the existing `ProjectStore`,
`SessionStore`, and `MessageStore` traits and is included in the `Store`
supertrait blanket impl:

```rust
trait CredentialStore: Send + Sync {
    fn credential_save(…) -> BoxFuture<…>;
    fn credential_load(…) -> BoxFuture<…>;
    fn credential_delete(…) -> BoxFuture<…>;
    fn credential_list(…) -> BoxFuture<…>;
}
trait Store: ProjectStore + SessionStore + MessageStore + CredentialStore {}
```

Both `InMemoryStore` and `FileStore` in `brain-stores` implement
`CredentialStore`. `FileStore` persists credentials as JSON files under
`{root}/credentials/{provider_name}.json` with mode 0600.

This replaces the original `OAuthTokenStore` trait + `FileOAuthTokenStore`
design. Credentials are now a first-class concept in the store layer rather
than an OAuth-specific concern.

## Dependency Choices

| Crate | Purpose | Why this one |
|-------|---------|-------------|
| `sha2` | PKCE S256 challenge | Standard RustCrypto crate, minimal deps |
| `base64` | base64url encoding for PKCE | Already widely used in the Rust ecosystem |
| `rand` | Cryptographic random for PKCE verifier | Standard, uses OS entropy by default |

We intentionally avoid `jsonwebtoken` for JWT decoding. The JWT is already
trusted (it came from the OAuth token endpoint), so we only need to decode
the payload — no signature verification. A simple base64-decode of the middle
segment + serde_json parse is sufficient and avoids pulling in a heavy
dependency with OpenSSL/ring.

## Two SSE Parsers

The `OpenAiProvider` and `OpenAiOAuthProvider` talk to **different API
surfaces**:

| | `OpenAiProvider` | `OpenAiOAuthProvider` |
|---|---|---|
| **Endpoint** | `api.openai.com/v1/chat/completions` | `chatgpt.com/backend-api/codex/responses` |
| **Format** | Chat Completions API | Responses API |
| **Auth** | API key Bearer | OAuth Bearer + `chatgpt-account-id` |
| **SSE module** | `openai_sse.rs` | `codex_sse.rs` |

### openai_sse.rs (Chat Completions)

Shared request/response types and SSE-to-`ChatChunk` conversion for the
standard OpenAI Chat Completions format. Used by `OpenAiProvider`.

### codex_sse.rs (Responses API)

OpenAI's OAuth tokens (from `auth.openai.com`) grant access to the proprietary
**Responses API** at `chatgpt.com/backend-api/codex/responses`, not the standard
Chat Completions API. This module implements:

- `ResponsesRequest` — maps `Message[]` to the Responses API format (`input`
  array + `instructions` field instead of `messages`)
- `build_responses_input()` — converts system messages to `instructions`,
  user/assistant messages to the `input` array
- `to_responses_tools()` — converts `ToolDef[]` to Responses API tool format
- `stream_from_response()` — parses Responses API SSE events
  (`response.output_text.delta`, `response.function_call_arguments.done`,
  `response.completed`) into `ChatChunk` items

Required headers: `OpenAI-Beta: responses=experimental`, `originator: brain`,
`accept: text/event-stream`, `chatgpt-account-id: {id}`.

## Device Flow (OpenAI Proprietary)

OpenAI's device code flow diverges from RFC 8628:

1. **JSON bodies** (not form-encoded) for both device-code and token-polling
   requests
2. Response returns `device_auth_id`, `user_code`, `interval` (string),
   `expires_at` (string) — no standard `verification_uri`
3. `device_verification_url` is hardcoded to
   `https://auth.openai.com/codex/device` (not returned by the API)
4. Token polling sends `device_auth_id` + `user_code` (not `device_code`)
5. A `User-Agent` header is included to avoid Cloudflare challenges

## Token Management

`OpenAiOAuthProvider` holds credentials behind `Arc<tokio::sync::Mutex<_>>`
for interior mutability during refresh. The mutex is only held briefly
(check expiry + swap credentials), never across network calls:

1. Lock mutex, check `needs_refresh()`
2. If refresh needed: clone current creds, release lock, do HTTP refresh,
   re-lock and swap in new creds + persist to store
3. Clone access_token + account_id, release lock
4. Use cloned values for the API request

This avoids holding the lock across the network call while still being
safe against concurrent `chat()` calls.

## Model Metadata

Both `OpenAiConfigPreset` and `OpenAiOAuthPreset` expose a
`models: &'static [ModelInfo]` field alongside `default_model`. `ModelInfo`
(in `brain-types/src/model.rs`) mirrors the
[models.dev](https://models.dev) schema:

- `id`, `name`, `family` — identification
- `reasoning: Option<&'static [&'static str]>` — supported effort levels
  (deviation from models.dev's `bool`; `None` = no reasoning, `Some(&["low",
  "medium", "high"])` = effort levels for runtime clamping)
- `tool_call`, `attachment`, `structured_output`, `temperature` — capabilities
- `cost: Option<ModelCost>` — per-million-token pricing (input, output,
  reasoning, cache_read, cache_write, audio)
- `limit: Option<ModelLimit>` — context, input, output token limits
- `input_modalities`, `output_modalities` — supported I/O types
- `knowledge`, `release_date`, `last_updated`, `open_weights`, `status`

All fields use `&'static str` / `&'static [...]` for zero-heap-allocation
`const` array compatibility. Data sourced from the
[models.dev](https://github.com/anomalyco/models.dev) repository.

The OpenAI API preset includes 12 models (gpt-4o, gpt-4o-mini, gpt-4.1
family, gpt-5 family, o3/o4 reasoning models). The OpenAI OAuth preset
includes 8 Codex models (gpt-5.x-codex variants, codex-mini-latest).

## Error Handling

A new `BrainError::Auth(String)` variant is added for authentication failures
(expired refresh token, revoked access, etc.). This is distinct from
`BrainError::Inference` because the recovery action is different: Auth errors
mean "re-login", while Inference errors mean "retry or change model".

A corresponding `BrainErrorCode::AuthFailed` is added.

## Runtime Integration

The implemented runtime model does not use a dedicated provider config section.
Instead:

- stored credentials live in the shared credential store
- provider selection flows through `agent.inference.provider`
- CLI and ACP provider discovery register `openai-oauth` automatically when
  stored OAuth credentials are present

If the config system later grows first-class provider declarations, that
belongs to `add-config-system` rather than this change.
