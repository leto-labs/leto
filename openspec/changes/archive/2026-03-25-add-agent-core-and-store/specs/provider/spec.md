## ADDED Requirements

### Requirement: Shared Tool Definitions May Preserve Output Schemas

The shared `provider` crate MUST allow tool definitions to carry an optional
output schema in addition to the input schema used for provider tool
registration.

#### Scenario: Shared tool definition round-trips with output schema
- **WHEN** a caller constructs a shared tool definition with an output schema
- **THEN** the shared type preserves that schema during serialization and deserialization
- **AND** provider adapters may ignore the output schema on the wire without rejecting the definition
