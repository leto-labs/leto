## MODIFIED Requirements

### Requirement: Native Tools Preset

The native tool preset MUST include an `apply_patch` tool for structured file
changes.

#### Scenario: Native preset includes apply_patch
- **WHEN** native tools are constructed
- **THEN** the preset includes `apply_patch`

### Requirement: ApplyPatch Tool

The `apply_patch` tool MUST accept patch text in the V4A patch-envelope format
used by Codex-family agents.

#### Scenario: Add file patch succeeds
- **WHEN** the caller submits a patch containing `*** Begin Patch`,
  `*** Add File:`, `+`-prefixed file contents, and `*** End Patch`
- **THEN** the tool creates the file

#### Scenario: Update patch succeeds
- **WHEN** the caller submits a patch containing `*** Update File:` and one or
  more `@@` hunks with context and `+` / `-` lines
- **THEN** the tool updates the file contents

#### Scenario: Delete patch succeeds
- **WHEN** the caller submits a patch containing `*** Delete File:`
- **THEN** the tool deletes the file

#### Scenario: Move patch succeeds
- **WHEN** the caller submits a patch containing `*** Update File:` followed by
  `*** Move to:`
- **THEN** the tool writes the updated contents at the new relative path and
  removes the original file

The `apply_patch` tool MUST reject malformed V4A input with explicit tool
errors.

#### Scenario: Missing envelope markers
- **WHEN** the patch omits `*** Begin Patch` or `*** End Patch`
- **THEN** the tool returns an error describing the missing markers

#### Scenario: Empty patch
- **WHEN** the patch contains no file operations
- **THEN** the tool returns an error indicating the patch is empty

#### Scenario: Absolute path
- **WHEN** a file operation references an absolute path
- **THEN** the tool returns an error indicating only relative paths are allowed
