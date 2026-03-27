## ADDED Requirements

### Requirement: Provider-OpenAI Supports A Standalone OAuth Surface

The `provider-openai` crate SHALL expose a standalone OAuth-backed provider
surface in addition to its direct API-key configuration path.

This SHALL include:

- browser and device login helpers
- refresh-token-based access-token renewal
- an OAuth-backed shared-provider adapter
- account-aware request shaping when required by the OpenAI OAuth backend

#### Scenario: Caller obtains persistence-ready OAuth credentials

- **WHEN** a caller completes a browser or device OAuth flow
- **THEN** `provider-openai` SHALL return credentials suitable for persistence
  by outer application layers

#### Scenario: OAuth-backed provider attaches account-aware headers

- **WHEN** the selected OAuth credential carries account context
- **THEN** the OAuth-backed provider SHALL attach the required account-aware
  request metadata in addition to bearer authentication

### Requirement: Provider-OpenAI Supports Shared Credential Pools

The `provider-openai` crate SHALL support managed credentials through the
shared `provider::CredentialPool`.

The shared OpenAI adapter and the OAuth-backed adapter SHALL both be
constructible from the shared pool while remaining usable without it through
their direct standalone constructors.

#### Scenario: Shared OpenAI adapter resolves credentials from a pool

- **WHEN** a caller constructs `OpenAiProvider` from the shared credential pool
- **THEN** each request SHALL resolve auth material from that pool
- **AND** terminal success or failure SHALL be reported back to the pool

#### Scenario: OAuth-backed adapter resolves credentials from a pool

- **WHEN** a caller constructs `OpenAiOAuthProvider` from the shared credential
  pool
- **THEN** each request SHALL resolve bearer authentication and account
  metadata from that pool
- **AND** terminal success or failure SHALL be reported back to the pool
