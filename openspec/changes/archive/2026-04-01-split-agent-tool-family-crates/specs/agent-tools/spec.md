# agent-tools Specification Delta

## MODIFIED Requirements

### Requirement: Agent Tools Capability Must Resolve To The Split Tool Family Architecture

The system MUST treat the old `agent-tools` capability as a legacy reference to
the split tool-family architecture rather than as a required monolithic crate.

The canonical tool surface SHALL now be provided by:

- `agent-tool` for shared tool contracts and the standard registry
- `agent-tool-files` for canonical file/workspace/search tools
- `agent-tool-process` for canonical process tools
- `agent-tool-web` for the reserved web-oriented tool family boundary

#### Scenario: Reader resolves the current tool architecture
- **WHEN** a caller or contributor looks up the canonical tool capability
- **THEN** the workspace directs them to the `agent-tool` plus `agent-tool-*`
  family crates
- **AND** it SHALL NOT require a monolithic `agent-tools` crate to assemble the
  default native tool surface
