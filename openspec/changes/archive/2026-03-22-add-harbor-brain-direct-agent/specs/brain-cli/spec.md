## ADDED Requirements

### Requirement: Non-Interactive Exec Command

The `brain` CLI SHALL support a non-interactive `exec` command for benchmark
and harness use.

The command SHALL:

- accept explicit `cwd`, `provider`, `model`, `loop`, and `output_dir`
- accept an explicit API key for the selected provider
- default to the Responses API when the selected OpenAI-compatible provider
  preset supports it
- allow the API surface to be explicitly overridden for debugging or
  compatibility
- execute one user instruction through the embedded runtime
- write structured run artifacts to the requested output directory

#### Scenario: Exec command writes Harbor-facing artifacts

- **WHEN** `brain exec` runs successfully
- **THEN** it SHALL write `run.json`, `events.jsonl`, and ATIF `trajectory.json`
- **AND** it SHALL exit successfully

#### Scenario: Exec command emits native ATIF trajectory

- **WHEN** Harbor runs the direct `brain` surface
- **THEN** `brain exec` SHALL emit Harbor-compatible ATIF `trajectory.json`
- **AND** Harbor-facing usage and cost metrics SHALL be sourced from `run.json`

#### Scenario: Exec command bypasses global brain home

- **WHEN** `brain exec` is used for a Harbor run
- **THEN** it SHALL NOT depend on stored `brain credentials`
- **AND** it SHALL NOT require `~/.brain/config.toml`
- **AND** it SHALL resolve only project-local config plus explicit CLI inputs
