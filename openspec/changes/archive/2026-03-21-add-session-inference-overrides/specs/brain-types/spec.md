# brain-types Specification

## ADDED Requirements

### Requirement: Session Inference Overrides

The session model SHALL support optional session-scoped inference overrides.

`Session` SHALL include:
`inference: Option<InferenceConfig>`

This field SHALL represent the session-local override layer rather than the
fully resolved effective inference config.

#### Scenario: New session starts without overrides

- **WHEN** a session is created
- **THEN** `Session.inference` SHALL be `None`

#### Scenario: Session stores partial inference override

- **WHEN** a session stores `InferenceConfig { model: Some("gpt-5"), temperature: Some(0.2), .. }`
- **THEN** only those fields SHALL be treated as session-local overrides

### Requirement: Session Updates Can Modify Inference

The session update model SHALL support setting or clearing session inference
overrides.

#### Scenario: Session update sets inference overrides

- **WHEN** a session update sets inference overrides
- **THEN** the store SHALL persist them on the session

#### Scenario: Session update clears inference overrides

- **WHEN** a session update clears inference overrides
- **THEN** the session SHALL fall back to project inference defaults
