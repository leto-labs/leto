## ADDED Requirements

### Requirement: Agent Store Persists Provider Credential Metadata

The `agent-store` crate SHALL persist provider credentials with enough metadata
to support OAuth refresh, account-aware provider requests, and credential
health tracking.

This SHALL include:

- multiple credentials per provider under a composite key
- API-key and OAuth-backed credential bodies
- OAuth access token, refresh token, client id, token endpoint, expiry, token
  type, scopes, and optional account context
- credential health state that can be updated independently of the credential
  body

#### Scenario: Stored OAuth credential round-trips with refresh metadata

- **WHEN** a caller persists and later reloads an OAuth-backed credential
- **THEN** the stored record SHALL preserve refresh, expiry, and account-aware
  metadata required by outer application layers

#### Scenario: Credential health updates do not replace the credential body

- **WHEN** a caller updates only the health state for one stored credential
- **THEN** the store SHALL preserve the existing API key or OAuth tokens
- **AND** only mutate the health metadata
