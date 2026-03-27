## ADDED Requirements

### Requirement: Provider SDK Supports Shared Credential Pools

The `provider` crate SHALL expose a standalone credential module for reusable
provider authentication and selection behavior.

This module SHALL include:

- provider-neutral credential material
- an in-memory `CredentialPool`
- selection-strategy interfaces and built-in strategies
- health tracking for successful and failed credential use

The shared credential module SHALL remain storeless and runtime-agnostic.

#### Scenario: Caller resolves credentials from a shared pool

- **WHEN** a caller registers multiple credentials under one provider name
- **THEN** the shared pool SHALL resolve one active credential for a request
- **AND** it SHALL return request-ready auth material without requiring any
  store dependency

#### Scenario: Sticky strategy preserves session affinity

- **WHEN** a caller uses the built-in sticky round-robin strategy with a
  session identifier
- **THEN** repeated resolution for the same session SHALL reuse the bound
  credential while it remains healthy

#### Scenario: Failed credential is rotated out

- **WHEN** a caller marks a selected credential as failed
- **THEN** the pool SHALL record the failure in health state
- **AND** future resolution SHALL avoid the failed credential while healthier
  alternatives exist
