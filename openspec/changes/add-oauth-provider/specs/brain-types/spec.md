# brain-types Delta Spec

## MODIFIED Requirements

### Requirement: Provider Trait
The `Provider` trait SHALL use the session-aware chat contract currently implemented in code: `chat(..., session_id: Option<Ulid>)`.
`OpenAiOAuthProvider` is an implementation that uses `session_id` when resolving credentials via the OAuth credential pool.

#### Scenario: OAuth provider implements same trait
- **WHEN** `OpenAiOAuthProvider` is constructed
- **THEN** it SHALL be usable as `Arc<dyn Provider>` interchangeably with `OpenAiProvider`, including session-bound calls

#### Scenario: ProviderRouter routes to OAuth provider
- **WHEN** a `ProviderRouter` includes both `OpenAiProvider` and `OpenAiOAuthProvider`
- **THEN** model-based routing SHALL work identically for both

### Requirement: Store Supertrait
The `Store` supertrait SHALL be extended to include `CredentialStore`:
```
trait Store: ProjectStore + SessionStore + MessageStore + CredentialStore {}
```

Existing `Store` implementations (`InMemoryStore`, `FileStore`) SHALL implement the new `CredentialStore` methods.

#### Scenario: Store implements CredentialStore
- **WHEN** a type implements `Store`
- **THEN** it SHALL also provide:
  - `credential_save`
  - `credential_load`
  - `credential_load_all`
  - `credential_delete`
  - `credential_update_health`
  - `credential_list`

### Requirement: Provider Presets Include Model Metadata
Both `OpenAiConfigPreset` and `OpenAiOAuthPreset` SHALL include a `models: &'static [ModelInfo]` field alongside `default_model`, providing structured metadata about supported models (capabilities, pricing, limits).

#### Scenario: Preset enumerates models
- **WHEN** `OpenAiConfigPreset::OPENAI.models` is accessed
- **THEN** it SHALL return a non-empty slice of `ModelInfo` entries with id, cost, limit, and capability data

#### Scenario: OAuth preset enumerates Codex models
- **WHEN** `OpenAiOAuthPreset::OPENAI.models` is accessed
- **THEN** it SHALL return a non-empty slice of `ModelInfo` entries for Codex-specific models
