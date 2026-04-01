# agent-core Specification Delta

## MODIFIED Requirements

### Requirement: Agent Core Must Assemble The V2 Runtime Stack

The system MUST provide an `agent-core` crate that acts as the app-facing
composition boundary above `agent-runtime`.

#### Scenario: Default local assembly
- **WHEN** a caller builds the default local core surface
- **THEN** the core registers a default loop
- **AND** installs a native tool executor composed from `agent-tool`,
  `agent-tool-files`, and `agent-tool-process`
- **AND** requires at least one real registered or discovered provider

#### Scenario: Default local assembly rejects missing providers
- **WHEN** a caller builds the default local core surface with no registered providers and no discoverable credentials
- **THEN** the build fails with a no-providers error

#### Scenario: Credential-based provider discovery
- **WHEN** the backing store contains OpenAI-compatible API-key or OAuth-backed credentials
- **THEN** the core registers corresponding providers during default local assembly
