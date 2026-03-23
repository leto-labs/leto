## ADDED Requirements

### Requirement: Native Runtime Remains Loop-Composable For Internal Tool Primitives

`BrainRuntimeNative` SHALL continue to expose a generic tool registry while
 allowing a selected loop to use registered tools as internal execution
 primitives without exposing those tool definitions to the provider.

#### Scenario: Terminus2Loop uses registered terminal tool internally

- **WHEN** the runtime resolves `Terminus2Loop`
- **THEN** the loop SHALL be able to find and invoke the registered
  terminal-session tool from the normal runtime tool registry
- **AND** the provider call for that loop MAY still receive an empty tool list

### Requirement: Native ATIF Export Supports Terminus2 Loop Metadata

The native ATIF export path SHALL preserve additive loop-specific metadata
 required by `Terminus2Loop`, including summarization handoff records and
 assistant reasoning content when present.

#### Scenario: Terminus2 turn exports handoff-aware ATIF steps

- **WHEN** a Terminus-2 turn performs summarization or completion confirmation
- **THEN** the exported ATIF trajectory SHALL preserve those additive steps in a
  Harbor-compatible structure
