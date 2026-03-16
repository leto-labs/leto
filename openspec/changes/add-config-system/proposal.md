# Proposal: add-config-system

## Why

brain currently has zero file-based configuration. All settings (API keys, model
selection, system prompts, tool presets) are passed programmatically or via env
vars. To function as a real coding agent SDK, brain needs a layered config system
that lets project maintainers and users declare per-project agent behavior
without touching code.

The coding-agent ecosystem has converged on a few conventions:

- **AGENTS.md** — a markdown file at the repo root (or nested in directories)
  that provides instructions/context to AI agents. Used by Codex, Claude Code,
  Cursor, and others.
- **.agents/ folder** — a directory for skills, commands, and other agent
  extensions. Some tools use `.claude/`, `.cursor/`, `.codex/` equivalents.
- **config.toml / config.yaml** — a structured config file for deeper settings
  (model routing, tool overrides, provider credentials). Codex uses
  `.codex/config.toml`.

brain should adopt these conventions rather than inventing new ones, while also
supporting a brain-specific config layer for settings that don't map to existing
standards.

## Architecture: Project-in-Store

Config is not a separate abstraction — it's a field on `Project`, which is a
first-class entity in the `Store` trait. This eliminates the need for a
`ConfigSource` trait or any config-specific persistence layer.

### Why not a ConfigSource trait?

The `Store` trait already abstracts platform-specific persistence:
- `FileStore` for CLI/TUI (filesystem)
- `InMemoryStore` for tests
- Future `IndexedDbStore` for web

A project's config is just another piece of data the Store persists. When you
load a project from the Store, you get its config. Different platforms *resolve*
config differently but all store it the same way via `Project.config`.

### Two-phase config resolution

1. **Base config** (project creation time): parse `.agents/config.toml`, resolve
   env vars, apply layered precedence. The result is a `ProjectConfig` stored as
   a field on `Project`. This is done by a utility function
   (`resolve_fs_config`), not by the Store.

2. **Nested AGENTS.md** (runtime, per-turn): directory-scoped agent instructions
   that vary based on what the agent is currently working on. These are resolved
   dynamically by the agent loop / Brain, not at project creation time.

### Construction flow by platform

**CLI/TUI:**
```rust
let config = resolve_fs_config(&cwd)?;
let project = store.project_create(Project::new("my-project", Some(cwd), config)).await?;
let brain = Brain::new(project, provider, store, agent_loop, tools);
```

**Web:**
```rust
let config = fetch_config_from_api(project_id).await?;
let project = store.project_create(Project::new("web-project", None, config)).await?;
let brain = Brain::new(project, provider, store, agent_loop, tools);
```

**Test:**
```rust
let project = store.project_create(Project::with_defaults("test")).await?;
let brain = Brain::new(project, provider, store, agent_loop, tools);
```

## What

### ProjectConfig struct

A `ProjectConfig` struct in `brain-types` that wraps `AgentConfig` via
`#[serde(flatten)]`. Pure data, no I/O. Extensible — future fields for provider
routing, tool presets, MCP servers will be added here.

### Project entity in Store

`Project` (defined in the store spec) includes `config: ProjectConfig`. When
you load a project from the Store, you get its config. Brain holds a `Project`
reference and derives `AgentConfig` from `project.config.agent`.

### Filesystem config resolution utility

A `resolve_fs_config(root: &Path) -> Result<ProjectConfig>` function (in
`brain-config` crate or module) for filesystem-based clients:

1. Walk upward from `root` to find `.agents/config.toml`
2. Parse TOML, resolve `$ENV_VAR` interpolation
3. Apply layered precedence (defaults → file → env → overrides)
4. Return resolved `ProjectConfig`

This is a plain function, not a trait. Web platforms have their own equivalent.

### Config schema

`.agents/config.toml` supports:
- `[model]` — default model, model aliases, fallback chains
- `[provider]` — provider configs (API keys reference env vars)
- `[tools]` — tool enable/disable, custom tool definitions, MCP server refs
- `[agent]` — system prompt, max iterations, temperature, loop settings
- `[session]` — store backend, session directory

### AGENTS.md integration (runtime)

Root-level `AGENTS.md` is prepended to the system prompt at session start.
Nested `AGENTS.md` files in subdirectories are injected as additional context
when the agent operates in those directories. This is a runtime/loop concern,
not a config-loading concern.

### No global config

By design, the SDK is project-local. If a user wants "global" behavior, they
can symlink or use env vars.

## Impact

- **Modified spec**: `store` (Project entity, decomposed traits)
- **New module/crate**: `brain-config` (filesystem config resolution utility)
- **Modifies**: `brain-types` (`ProjectConfig` struct, `Project` struct)
- **Modifies**: `brain-engine` (Brain holds Project, derives AgentConfig)
- **New spec**: `config-system`
- **No breaking changes to Store consumers**: `dyn Store` still works,
  sub-traits (`ProjectStore`, `SessionStore`, `MessageStore`) available for
  granular bounds

## Change Dependencies

- **Requires**: none (independent, can start immediately)
- **Required by**: `add-oauth-provider` (config declares OAuth providers, token store path)
- **Required by**: `add-mcp-client` (config declares MCP servers via `[[mcp.servers]]`)
- **Required by**: `add-brain-cli` (uses project-driven initialization)

## Crate Dependencies

- `toml` crate for config parsing (used by `resolve_fs_config`)
- Markdown parsing (lightweight — for AGENTS.md frontmatter, runtime only)
