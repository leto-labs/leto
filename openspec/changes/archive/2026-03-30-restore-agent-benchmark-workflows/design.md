# Design: restore-agent-benchmark-workflows

## Decision: Harbor Uses Live Repo-Owned Agent Surfaces

The live benchmark runner should expose two repo-owned migrated surfaces:

- direct `agent`
- ACP-backed `agent-acp`

These replace the deleted legacy `brain` and `brain-acp` Harbor helpers.

## Decision: ACP Smoke Validation Keeps An Explicit Mock Lane

The migrated `acpx` smoke workflow should keep an explicit mock ACP backend.
That gives the repo a stable local interoperability harness that does not
depend on external model credentials or network availability.

## Decision: Harbor Session Model Mutation Uses Supported Config Options

The migrated Harbor ACP integration should use the supported
`session/set_config_option` path for `model` and `loop` rather than preserving
the deleted legacy `session/set_model` consumer path.
