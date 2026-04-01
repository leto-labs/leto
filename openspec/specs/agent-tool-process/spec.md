# agent-tool-process Specification

## Purpose
TBD - created by archiving change split-agent-tool-family-crates. Update Purpose after archive.
## Requirements
### Requirement: Agent Tool Process Must Provide Canonical Process Tools

The system MUST provide an `agent-tool-process` crate that owns the first-party
process-oriented tool implementations.

This SHALL include the canonical `shell` tool and its swappable driver
contract.

#### Scenario: Native process tool family registers its canonical tools
- **WHEN** a caller builds or extends the native process tool family registry
- **THEN** the resulting tool definitions include the canonical `shell` tool
- **AND** the tool executes through typed request/response contracts and a
  swappable process driver

