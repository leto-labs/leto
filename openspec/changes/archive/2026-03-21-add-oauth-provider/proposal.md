# Proposal: add-oauth-provider

## Why

brain's Provider trait currently only supports static API key authentication
(Bearer token). This works for pay-per-token API access but excludes a major
use case: **using existing subscription credits**.

OpenAI offers Pro/Plus/Team subscriptions that provide generous token budgets
through their consumer products (ChatGPT, Codex). These subscriptions use OAuth
2.0 with PKCE for authentication, not API keys. A user with an OpenAI Pro
subscription ($200/mo) gets far more capacity than typical API spend, but brain
can't tap into it because it only knows Bearer tokens.

Anthropic's consumer auth is more locked down (they actively restrict
third-party OAuth clients), so we skip that for now and focus on OpenAI.

## Why a Separate Provider (Not an Auth Enum)

The OAuth/subscription path is not just a different auth method on the same
API — it's a **fundamentally different API surface**:

- **Different API**: Responses API (`chatgpt.com/backend-api/codex/responses`)
  instead of Chat Completions API (`api.openai.com/v1/chat/completions`), with
  distinct request/response formats (e.g. `input` array + `instructions` instead
  of `messages`)
- **Extra required headers**: `chatgpt-account-id`, `OpenAI-Beta`,
  `originator`, `accept` on every request
- **Different models**: Codex-specific models (gpt-5.x-codex) not available on
  the standard API
- **Different rate limits**: subscription quota vs per-token billing
- **Different SSE parsing**: Responses API emits events like
  `response.output_text.delta` and `response.function_call_arguments.done`
  instead of standard Chat Completions `delta` chunks

Trying to cram this into the existing `OpenAiProvider` via an `Auth` enum
would mean internal branching on every request. Instead:

- **`OpenAiProvider`** stays clean — API key, standard Chat Completions API
- **`OpenAiOAuthProvider`** (new) — handles OAuth flow, token management,
  account ID header, Responses API format
- Both implement `Provider`, both are interchangeable from Brain's perspective
- `ProviderRouter` routes to either based on model name
- Shared OAuth plumbing (PKCE, refresh, device flow) lives in a common module

## Reference Implementation: OpenCode

OpenCode (`repocache/anomalyco/opencode`, `plugin/codex.ts`) has a mature
OAuth implementation for OpenAI that we use as our primary reference.

### OpenAI OAuth Details (from OpenCode)

- **Issuer**: `https://auth.openai.com`
- **Authorize endpoint**: `https://auth.openai.com/oauth/authorize`
- **Token endpoint**: `https://auth.openai.com/oauth/token`
- **Device code endpoint**: `https://auth.openai.com/api/accounts/deviceauth/usercode`
- **Device token endpoint**: `https://auth.openai.com/api/accounts/deviceauth/token`
- **Device verification URL**: `https://auth.openai.com/codex/device` (hardcoded, not returned by API)
- **Client ID**: `app_EMoamEEZ73f0CkXaXp7hrann` (OpenAI's public client ID, no secret)
- **Scopes**: `openid profile email offline_access`
- **PKCE**: `S256` code challenge method
- **API base**: `https://chatgpt.com/backend-api/codex` (Responses API, not Chat Completions)

### Two Auth Flows

1. **Browser flow (auth code + PKCE)**:
   - Start localhost callback server (OpenCode uses port 1455)
   - Generate PKCE code verifier (43-char random) + challenge (SHA256 + base64url)
   - Open browser to authorize URL with `client_id`, `redirect_uri`, `code_challenge`, `scope`
   - Receive callback with auth code
   - Exchange code for access + refresh tokens at token endpoint
   - 5-minute timeout

2. **Device code flow (OpenAI proprietary)**:
   - POST JSON to device code endpoint → get `device_auth_id`, `user_code`, `interval`, `expires_at`
   - Display code + verification URL to user
   - Poll device token endpoint with JSON body (`device_auth_id` + `user_code`)
   - Uses `redirect_uri: https://auth.openai.com/deviceauth/callback`

### Token Refresh

- POST to `/oauth/token` with `grant_type: "refresh_token"`, `client_id`, `refresh_token`
- No client secret required (public client)
- Done before each request when `expires < now`

### Extra Headers

- `chatgpt-account-id` header extracted from JWT claims (`chatgpt_account_id`
  or `organizations[0].id`) — required for Codex models
- `OpenAI-Beta: responses=experimental`
- `originator: brain`

## What

### Credential architecture (`brain-types`, `brain-stores`)

Credentials are modelled as a `ProviderCredential` sum type in `brain-types`:

```rust
enum ProviderCredential {
    ApiKey { api_key: String },
    OAuth(OAuthCredentials),
}
```

A `CredentialStore` trait is added to `brain-types/traits.rs` and included in
the `Store` supertrait blanket impl, so `InMemoryStore` and `FileStore` both
handle credential persistence uniformly.

### Shared OAuth module (`brain-providers/oauth/`)

Reusable OAuth plumbing, not specific to any single provider:

- `OAuthCredentials` struct (in brain-types, re-exported)
- PKCE utilities (verifier generation, S256 challenge)
- Browser auth flow (localhost callback server + code exchange)
- Device code flow (OpenAI proprietary JSON-based polling)
- Token refresh logic
- JWT claim extraction (base64 decode, no sig verification)

This module is provider-agnostic. Future OAuth providers (GitHub Copilot,
Google, etc.) would reuse the same plumbing with different endpoints and
client IDs.

### Codex SSE module (`brain-providers/codex_sse.rs`)

A dedicated SSE parser for the Responses API, separate from the Chat
Completions parser (`openai_sse.rs`). Handles:

- `ResponsesRequest` construction (mapping `Message[]` to `input` + `instructions`)
- Tool definition conversion to Responses API format
- Parsing Responses API SSE events into `ChatChunk`

### `OpenAiOAuthProvider` (new)

A new provider in `brain-providers/openai_oauth/` that implements `Provider`:

- Constructs from `OAuthCredentials` + `CredentialStore`
- On each `chat()` call:
  1. Check token expiry, refresh if needed
  2. Set `Authorization: Bearer {access_token}` header
  3. Set `chatgpt-account-id`, `OpenAI-Beta`, `originator` headers
  4. Build Responses API request from messages + tools
  5. Stream the SSE response via `codex_sse`, yielding `ChatChunk` items
- Provides login helpers: `login_browser()`, `login_device()`
- Provides a preset: `OpenAiOAuthPreset` with endpoints, client ID, and models

### Model metadata (`brain-types/model.rs`)

Both `OpenAiConfigPreset` and `OpenAiOAuthPreset` expose a
`models: &'static [ModelInfo]` field alongside `default_model`. `ModelInfo`
mirrors the [models.dev](https://models.dev) schema with `reasoning` changed
from `bool` to `Option<&'static [&'static str]>` (effort level list) for
runtime clamping. Data sourced from the models.dev repository.

### `OpenAiOAuthPreset`

```rust
struct OpenAiOAuthPreset {
    name: &'static str,
    issuer: &'static str,
    authorize_url: &'static str,
    token_url: &'static str,
    device_code_url: &'static str,
    device_token_url: &'static str,
    device_verification_url: &'static str,
    device_redirect_uri: &'static str,
    client_id: &'static str,
    scopes: &'static str,
    api_base_url: &'static str,
    default_model: &'static str,
    callback_port: u16,
    models: &'static [ModelInfo],
}
```

Single const preset for now (OpenAI). Extensible for future providers.

### Runtime discovery

OAuth-backed providers are discovered from stored credentials at runtime. The
current config surface selects the default provider through
`agent.inference.provider`, and the CLI / ACP runtime registers
`openai-oauth` automatically when stored OAuth credentials are present.

Richer provider-declaration syntax in `.agents/config.toml` belongs to
`add-config-system`, not to this change.

### What we're NOT doing

- Modifying `OpenAiProvider` — it stays API-key-only, Chat Completions API
- `Auth` enum — providers manage their own auth, no shared abstraction
- Anthropic OAuth (locked down, skip for now)
- GitHub Copilot OAuth (different flow, lower priority)
- Generic OAuth provider factory (start specific, generalize later)
- MCP OAuth (separate concern, handled by add-mcp-client if needed)

## Change Dependencies

- **Requires**: none
- **Related**: `add-config-system` may later add richer provider-declaration syntax, but the OAuth provider and stored-credential discovery already work with the current config/runtime model

## Impact

- **New types**: `ProviderCredential`, `OAuthCredentials`, `CredentialStore`,
  `ModelInfo`, `ModelCost`, `ModelLimit` in `brain-types`
- **New provider**: `OpenAiOAuthProvider` in `brain-providers` (feature-gated: `openai-oauth`)
- **New modules**: `brain-providers/oauth/` (shared OAuth plumbing),
  `brain-providers/codex_sse.rs` (Responses API SSE)
- **Modified**: `Store` supertrait gains `CredentialStore`, `InMemoryStore` and
  `FileStore` implement it. `OpenAiConfigPreset` and `OpenAiOAuthPreset` gain
  `models` field.
- **Dependencies**: `sha2` + `base64` for PKCE (no `jsonwebtoken` — manual base64
  JWT decode instead)
- **No breaking changes**: `OpenAiProvider` is untouched, new provider is additive
