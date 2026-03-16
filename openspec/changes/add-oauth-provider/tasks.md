# Tasks: add-oauth-provider

## Implementation Checklist

### Credential architecture (`brain-types`, `brain-stores`)
- [x] Define `ProviderCredential` enum (ApiKey | OAuth) in `brain-types/credential.rs`
- [x] Define `OAuthCredentials` struct: access_token, refresh_token, expires_at, client_id, token_endpoint, account_id
- [x] Define `CredentialStore` trait in `brain-types/traits.rs`: save, load, delete, list
- [x] Add `CredentialStore` to `Store` supertrait blanket impl
- [x] Implement `CredentialStore` for `InMemoryStore` (brain-stores)
- [x] Implement `CredentialStore` for `FileStore` (brain-stores, JSON files, mode 0600)

### Model metadata (`brain-types`, presets)
- [x] Define `ModelInfo`, `ModelCost`, `ModelLimit` in `brain-types/model.rs`
- [x] Add `models: &'static [ModelInfo]` field to `OpenAiConfigPreset`
- [x] Populate OpenAI API preset with 12 key models from models.dev
- [x] Add `models: &'static [ModelInfo]` field to `OpenAiOAuthPreset`
- [x] Populate OpenAI OAuth preset with 8 Codex models from models.dev

### Shared OAuth module (`brain-providers/oauth/`)
- [x] Implement PKCE verifier generation (43-char cryptographic random)
- [x] Implement PKCE S256 challenge (SHA256 + base64url)
- [x] Implement browser auth flow: localhost callback server, authorization URL, code exchange
- [x] Implement device code flow: OpenAI proprietary JSON-based flow with device_auth_id polling
- [x] Implement token refresh: POST grant_type=refresh_token, persist updated creds
- [x] Implement JWT claim extraction for account ID (chatgpt_account_id or organizations[0].id)

### Codex SSE module (`brain-providers/codex_sse.rs`)
- [x] Define `ResponsesRequest` and related types for Responses API format
- [x] Implement `build_responses_input()` — convert Message[] to instructions + input array
- [x] Implement `to_responses_tools()` — convert ToolDef[] to Responses API tools
- [x] Implement `stream_from_response()` — parse Responses API SSE events into ChatChunk

### OpenAI OAuth constants
- [x] Define `OpenAiOAuthPreset` struct with all endpoint URLs, client ID, scopes, models
- [x] Define const preset with OpenAI's known values (issuer, authorize, token, device endpoints)
- [x] Client ID: `app_EMoamEEZ73f0CkXaXp7hrann`
- [x] Scopes: `openid profile email offline_access`

### OpenAiOAuthProvider
- [x] Create `brain-providers/openai_oauth/` module (feature-gated: `openai-oauth`)
- [x] Implement `OpenAiOAuthProvider` struct holding OAuthCredentials, CredentialStore, account_id
- [x] Implement `Provider` trait for `OpenAiOAuthProvider` (using Responses API via codex_sse)
- [x] On chat(): check token expiry → refresh if needed → set Bearer + chatgpt-account-id headers → request Responses API
- [x] Implement `login_browser()` → run browser flow → persist creds → return provider
- [x] Implement `login_device()` → run device flow → persist creds → return provider
- [x] Implement `from_stored(store, provider_name)` → load existing creds → return provider

### Additional
- [x] Add `BrainError::Auth` variant + `BrainErrorCode::AuthFailed`
- [x] Extract shared Chat Completions SSE parsing into `openai_sse` module (used by `openai`)
- [x] Write `design.md` for change

### Config integration
- [ ] Support `type = "openai-oauth"` in provider config section (blocked on add-config-system)
- [ ] Auto-discover stored OAuth tokens when building from config (blocked on add-config-system)

### Tests
- [x] Unit tests for PKCE generation and verification
- [x] Unit tests for JWT claim extraction
- [x] Unit tests for OAuthCredentials expiry / needs_refresh
- [x] Unit tests for CredentialStore (save/load/delete via FileStore)
- [ ] Integration test with mock OAuth server (browser flow)
- [ ] Integration test with mock device code server
