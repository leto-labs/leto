## ADDED Requirements

### Requirement: Agent Config Supports Tool Output Budget

Project and session agent settings SHALL include a global byte budget for
model-visible tool output.

#### Scenario: Project config overrides tool output cap
- **WHEN** `.agents/config.toml` sets `agent.tool_output_max_bytes`
- **THEN** the resolved `AgentConfig` SHALL carry that value
- **AND** loop implementations SHALL use it when truncating tool results before
  they re-enter model-visible session history
