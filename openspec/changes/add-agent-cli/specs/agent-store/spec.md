# agent-store Specification Delta

## ADDED Requirements

### Requirement: Agent Store Must Resolve Project Bootstrap Files

The `agent-store` crate MUST provide reusable loading for project-local
bootstrap files used by refactored local surfaces.

This MUST include:

- discovery of `.agents/config.toml` by walking up from a project root
- parsing the `[agent]` table into the durable `ProjectConfig` shape
- `.agents/AGENTS.md` fallback to the project system prompt when no explicit
  prompt is configured

#### Scenario: Store resolves project bootstrap defaults from filesystem inputs
- **WHEN** a caller asks `agent-store` to resolve project bootstrap state for a
  project tree
- **THEN** it SHALL load the project defaults from `.agents/config.toml`
- **AND** apply `.agents/AGENTS.md` as the prompt fallback when needed
