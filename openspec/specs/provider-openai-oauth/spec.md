# provider-openai-oauth Specification

## Purpose
OAuth credential lifecycle and OpenAI Codex-compatible provider integration in
the current `provider-openai` + `agent-store` stack.

## Requirements

### Requirement: ProviderCredential
The system SHALL define a `ProviderCredential` enum in the shared store
credential model as a tagged union:
- `ApiKey { api_key: String }` — static API key
- `OAuth(OAuthCredentials)` — OAuth 2.0 tokens

It SHALL be serializable (Serialize + Deserialize) with `#[serde(tag = "type")]` for persistence.

#### Scenario: API key roundtrip
- **WHEN** `ProviderCredential::ApiKey { api_key: "sk-123" }` is serialized and deserialized
- **THEN** the result SHALL match the original, with JSON containing `"type":"api_key"`

#### Scenario: OAuth roundtrip
- **WHEN** `ProviderCredential::OAuth(creds)` is serialized and deserialized
- **THEN** the result SHALL match the original, with JSON containing `"type":"o_auth"`

### Requirement: OAuthCredentials
The system SHALL define an `OAuthCredentials` struct in the shared store
credential model with:
- `access_token: String` — current access token
- `refresh_token: String` — token used to obtain new access tokens
- `expires_at: DateTime<Utc>` — when the access token expires
- `client_id: String` — OAuth client ID (needed for refresh, public client)
- `token_endpoint: String` — URL for token refresh requests
- `account_id: Option<String>` — provider-specific account ID (e.g., chatgpt-account-id)

It SHALL be serializable (Serialize + Deserialize) for persistence.

#### Scenario: Token not expired
- **WHEN** `expires_at` is more than 60 seconds in the future
- **THEN** `needs_refresh()` SHALL return false

#### Scenario: Token near expiry
- **WHEN** `expires_at` is within 60 seconds of now
- **THEN** `needs_refresh()` SHALL return true (proactive refresh buffer)

#### Scenario: Token expired
- **WHEN** `expires_at` is in the past
- **THEN** `needs_refresh()` SHALL return true

### Requirement: CredentialStore
The system SHALL define a `CredentialStore` trait in `agent-store` for
persisting and loading provider credentials:
- `credential_save(provider_name: &str, entry: &CredentialEntry) -> BoxFuture<Result<()>>`
- `credential_load(provider_name: &str, credential_id: &str) -> BoxFuture<Result<Option<CredentialEntry>>>`
- `credential_load_all(provider_name: &str) -> BoxFuture<Result<Vec<CredentialEntry>>>`
- `credential_delete(provider_name: &str, credential_id: &str) -> BoxFuture<Result<()>>`
- `credential_update_health(provider_name: &str, credential_id: &str, health: &CredentialHealth) -> BoxFuture<Result<()>>`
- `credential_list() -> BoxFuture<Result<Vec<(String, CredentialEntry)>>>`

The `Store` supertrait SHALL include `CredentialStore` in its blanket impl:
```
trait Store: ProjectStore + SessionStore + MessageStore + CredentialStore {}
```

Both `InMemoryStore` and `FileStore` in `agent-store` SHALL implement
`CredentialStore`. `FileStore` SHALL persist credentials as JSON files under
`{root}/credentials/{provider_name}/{credential_id}.json` with restricted
permissions (mode 0600 on Unix).

#### Scenario: Persist and reload
- **WHEN** credentials are saved via `credential_save("openai", entry)`
- **THEN** `credential_load("openai", entry.id)` SHALL return those credentials

#### Scenario: No stored credentials
- **WHEN** no credentials have been saved for a provider
- **THEN** `credential_load("openai", "unknown")` SHALL return `Ok(None)`

#### Scenario: List credentials
- **WHEN** credentials exist for multiple providers
- **THEN** `credential_list()` SHALL return all (name, credential) pairs

### Requirement: ModelInfo
The system SHALL define `ModelInfo`, `ModelCost`, and `ModelLimit` structs in
the shared `provider` crate, closely mirroring the
[models.dev](https://models.dev) schema.

`ModelInfo` SHALL include:
- `id`, `name`, `family` — identification fields
- `reasoning: Option<&'static [&'static str]>` — `None` for non-reasoning models; `Some(&["low", "medium", "high"])` listing supported effort levels (deviation from models.dev `bool`)
- `tool_call`, `attachment`, `structured_output`, `temperature` — capability flags
- `cost: Option<ModelCost>` — per-million-token pricing
- `limit: Option<ModelLimit>` — token limits (context, input, output)
- `input_modalities`, `output_modalities` — supported I/O types
- `knowledge`, `release_date`, `last_updated`, `open_weights`, `status` — metadata

All fields SHALL use `&'static str` / `&'static [...]` for zero-heap-allocation `const` array compatibility.

#### Scenario: Reasoning model lookup
- **WHEN** a model's `reasoning` field is `Some(&["low", "medium", "high"])`
- **THEN** the caller SHALL be able to validate/clamp reasoning effort levels at runtime

#### Scenario: Non-reasoning model
- **WHEN** a model's `reasoning` field is `None`
- **THEN** the caller SHALL know reasoning parameters are not applicable

### Requirement: PKCE Support
The system SHALL implement PKCE (Proof Key for Code Exchange) utilities:
- Generate a cryptographically random code verifier (43 characters, unreserved URI characters)
- Compute the code challenge as `base64url(SHA256(verifier))` using method `S256`

#### Scenario: PKCE generation
- **WHEN** `pkce_verifier()` is called
- **THEN** it SHALL return a tuple of (verifier, challenge) where the challenge is the S256 hash of the verifier

### Requirement: Browser OAuth Flow
The system SHALL implement an OAuth 2.0 Authorization Code + PKCE flow for interactive environments:
1. Generate PKCE code verifier and challenge
2. Start a temporary localhost HTTP server on a configurable port (default 1455)
3. Construct the authorization URL with client_id, redirect_uri, code_challenge, scope, response_type=code
4. Invoke a caller-provided callback with the URL (for opening in browser)
5. Wait for the authorization callback (with configurable timeout, default 5 minutes)
6. Extract the authorization code from the callback
7. Exchange the code + verifier for tokens at the token endpoint
8. Return `OAuthCredentials`

#### Scenario: Successful browser login
- **WHEN** the user completes authorization in their browser
- **THEN** the system SHALL receive the auth code via localhost callback
- **AND** exchange it for access + refresh tokens
- **AND** return valid `OAuthCredentials`

#### Scenario: User does not complete login
- **WHEN** the callback is not received within the timeout period
- **THEN** the system SHALL return an error indicating timeout

#### Scenario: Port conflict
- **WHEN** the callback port is already in use
- **THEN** the system SHALL return an error with a clear message

### Requirement: Device Code Flow
The system SHALL implement OpenAI's proprietary device code flow for headless environments:
1. POST JSON to the device authorization endpoint → receive `device_auth_id`, `user_code`, `interval`, `expires_at`
2. Construct verification URL by appending `?user_code={code}` to the hardcoded device verification URL
3. Invoke a caller-provided callback with the user code and full verification URL (for display)
4. Poll the device token endpoint at the specified interval with JSON body (`device_auth_id` + `user_code`)
5. Handle polling states: authorization_pending, slow_down, expired_token
6. On success, return `OAuthCredentials`

A `User-Agent` header SHALL be included on all requests to mitigate Cloudflare challenges.

#### Scenario: Successful device login
- **WHEN** the user enters the code on the verification URL
- **THEN** polling SHALL detect the authorization and return valid `OAuthCredentials`

#### Scenario: User does not authorize
- **WHEN** the device code expires before the user authorizes
- **THEN** the system SHALL return an error indicating the code expired

#### Scenario: Slow down response
- **WHEN** the token endpoint responds with `slow_down`
- **THEN** the system SHALL increase the polling interval by 5 seconds

### Requirement: Token Refresh
The system SHALL implement a token refresh function that:
1. POSTs to the token endpoint with `grant_type=refresh_token`, `client_id`, `refresh_token`
2. Parses the new access token, refresh token, and expiry from the response
3. Returns updated `OAuthCredentials`

The function is stateless — callers (providers) are responsible for calling it when needed and persisting the result via `CredentialStore`.

#### Scenario: Successful refresh
- **WHEN** `refresh_token(creds)` is called with valid credentials
- **THEN** it SHALL return new `OAuthCredentials` with a fresh access token and updated expiry

#### Scenario: Refresh token rejected
- **WHEN** the refresh token is revoked or expired
- **THEN** `refresh_token()` SHALL return an error indicating re-authentication is required

### Requirement: JWT Account ID Extraction
The system SHALL provide a utility to extract provider-specific account IDs from JWT access tokens. For OpenAI, it SHALL decode the JWT payload (without signature verification, since the token is already trusted) and look for `chatgpt_account_id`, falling back to `organizations[0].id`.

#### Scenario: Account ID from chatgpt_account_id
- **WHEN** the JWT contains a `chatgpt_account_id` claim
- **THEN** `extract_account_id(token)` SHALL return that value

#### Scenario: Fallback to organization ID
- **WHEN** the JWT does not contain `chatgpt_account_id` but has `organizations`
- **THEN** it SHALL return the first organization's `id`

#### Scenario: No account ID available
- **WHEN** the JWT contains neither claim
- **THEN** it SHALL return `None`

### Requirement: Codex SSE Parser
The system SHALL provide a `codex_sse` module for parsing OpenAI's Responses API SSE stream, separate from the Chat Completions SSE parser (`openai_sse`). It SHALL:
- Define `ResponsesRequest` and related types for the Responses API format
- Convert `Message[]` to Responses API `input` array + `instructions` string via `build_responses_input()`
- Convert `ToolDef[]` to Responses API tool format via `to_responses_tools()`
- Parse SSE events (`response.output_text.delta`, `response.function_call_arguments.done`, `response.completed`) into `ChatChunk` items via `stream_from_response()`

#### Scenario: Text delta streaming
- **WHEN** the SSE stream contains `response.output_text.delta` events
- **THEN** they SHALL be converted to `ChatChunk::Delta` items

#### Scenario: Tool call completion
- **WHEN** the SSE stream contains a `response.function_call_arguments.done` event
- **THEN** it SHALL be converted to a `ChatChunk::ToolCall` with name, call_id, and parsed arguments

#### Scenario: Response completed
- **WHEN** the SSE stream contains a `response.completed` event with usage
- **THEN** it SHALL be converted to `ChatChunk::Done` with token usage

### Requirement: OpenAiOAuthProvider
The system SHALL provide an `OpenAiOAuthProvider` struct in `provider-openai`
that implements the shared provider trait. It is a separate provider from
`OpenAiProvider`, purpose-built for OpenAI subscription/OAuth access via the
Responses API.

It SHALL hold:
- `OAuthCredentials` (behind `Arc<tokio::sync::Mutex<_>>` for interior mutability during refresh)
- A `CredentialStore` (for persisting refreshed tokens)
- `account_id: String` (the chatgpt-account-id)
- HTTP client for API requests

On each `chat()` call it SHALL:
1. Check if the access token needs refresh; if so, refresh and persist
2. Set `Authorization: Bearer {access_token}` header
3. Set `chatgpt-account-id: {account_id}` header
4. Set `OpenAI-Beta: responses=experimental` and `originator: brain` headers
5. Build a `ResponsesRequest` from messages and tools via `codex_sse`
6. POST to `{api_base_url}/responses`
7. Stream the SSE response via `codex_sse`, yielding `ChatChunk` items

#### Scenario: Chat with valid token
- **WHEN** `chat()` is called and the token is valid
- **THEN** it SHALL make the Responses API call with proper headers and stream the response

#### Scenario: Chat with expired token
- **WHEN** `chat()` is called and the token needs refresh
- **THEN** it SHALL refresh the token, persist it, then proceed with the API call

#### Scenario: Refresh fails during chat
- **WHEN** token refresh fails (e.g., refresh token expired)
- **THEN** `chat()` SHALL return `BrainError::Auth` indicating re-login is needed

### Requirement: OpenAiOAuthPreset
The system SHALL define an `OpenAiOAuthPreset` struct with all OAuth endpoint URLs, client ID, scopes, API base URL, default model, callback port, and model list. A single const preset SHALL be provided for OpenAI with the known values:
- Issuer: `https://auth.openai.com`
- Authorize: `https://auth.openai.com/oauth/authorize`
- Token: `https://auth.openai.com/oauth/token`
- Device code: `https://auth.openai.com/api/accounts/deviceauth/usercode`
- Device token: `https://auth.openai.com/api/accounts/deviceauth/token`
- Device verification URL: `https://auth.openai.com/codex/device`
- Device redirect URI: `https://auth.openai.com/deviceauth/callback`
- Client ID: `app_EMoamEEZ73f0CkXaXp7hrann`
- Scopes: `openid profile email offline_access`
- API base URL: `https://chatgpt.com/backend-api/codex`
- Default model: `gpt-5.3-codex`
- Callback port: 1455
- Models: static array of Codex model metadata

#### Scenario: Preset provides endpoints
- **WHEN** `OpenAiOAuthPreset::OPENAI` is accessed
- **THEN** it SHALL contain all the known OpenAI OAuth endpoint URLs, client ID, and model list

### Requirement: Login Helpers
The system SHALL provide convenience methods on `OpenAiOAuthProvider` for the login flow:
- `login_browser(preset, store, on_prompt) -> Result<OpenAiOAuthProvider>` — runs browser flow, persists tokens, returns a ready provider
- `login_device(preset, store, on_user_prompt) -> Result<OpenAiOAuthProvider>` — runs device flow with a caller-provided function to display the user code, persists tokens, returns a ready provider
- `from_stored(store, preset) -> Result<Option<OpenAiOAuthProvider>>` — loads stored tokens, returns a provider if tokens exist

#### Scenario: Login and use
- **WHEN** `login_browser(...)` completes successfully
- **THEN** it SHALL return an `OpenAiOAuthProvider` ready for `chat()` calls
- **AND** tokens SHALL be persisted in the store

#### Scenario: Restore from stored tokens
- **WHEN** `from_stored(store, preset)` is called and tokens exist in the store
- **THEN** it SHALL return `Some(provider)` with the stored credentials

### Requirement: Feature Gate
The `OpenAiOAuthProvider`, `codex_sse`, and all OAuth plumbing SHALL be gated behind the `openai-oauth` feature flag (off by default). Building without the feature SHALL not compile any OAuth dependencies.

#### Scenario: Feature disabled
- **WHEN** `provider-openai` is compiled without the `openai-oauth` feature
- **THEN** `OpenAiOAuthProvider`, OAuth utilities, and related types SHALL not be available
