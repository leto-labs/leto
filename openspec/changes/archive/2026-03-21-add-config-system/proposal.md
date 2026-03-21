# Proposal: add-config-system

## Why

brain now has a working file-based config system, but this change proposal still
describes a much broader future schema than what actually shipped. It should be
re-scoped to the implemented behavior so OpenSpec reflects reality and the
change can be archived cleanly.

The coding-agent ecosystem has converged on a few conventions:

- **AGENTS.md** — a markdown file at the repo root (or nested in directories)
  that provides instructions/context to AI agents. Used by Codex, Claude Code,
  Cursor, and others.
- **.agents/ folder** — a directory for skills, commands, and other agent
  extensions. Some tools use `.claude/`, `.cursor/`, `.codex/` equivalents.
- **config.toml / config.yaml** — a structured config file for deeper settings
  (model routing, tool overrides, provider credentials). Codex uses
  `.codex/config.toml`.

The implemented system already covers the important local-runtime baseline:
layered agent config plus AGENTS-based prompt/context discovery.

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
let project = store.project_create(Project::new(Some("my-project".into()), Some(cwd), config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Web:**
```rust
let config = fetch_config_from_api(project_id).await?;
let project = store.project_create(Project::new(Some("web-project".into()), None, config)).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
brain.run(&project, &transport).await?;
```

**Test:**
```rust
let project = store.project_create(Project::with_defaults("test")).await?;
let brain = Brain::new(provider, store, agent_loop, tools);
```

## What Changes

### ProjectConfig struct

A `ProjectConfig` struct in `brain-types` that wraps `AgentConfig` via
`#[serde(flatten)]`. Pure data, no I/O. Extensible — future fields for provider
routing, tool presets, MCP servers will be added here.

### Project entity in Store

`Project` (defined in the store spec) includes `config: ProjectConfig`. When
you load a project from the Store, you get its config. Callers pass `Project`
context into Brain or runtime entrypoints, and execution derives `AgentConfig`
from `project.config.agent`.

### Filesystem config resolution utility

A `resolve_fs_config(root: &Path) -> Result<ProjectConfig>` function (in
`brain-config` crate or module) for filesystem-based clients:

1. Walk upward from `root` to find `.agents/config.toml`
2. Parse TOML, resolve `$ENV_VAR` interpolation
3. Apply layered precedence (defaults → global `~/.brain/config.toml` → project `.agents/config.toml`)
4. Return resolved `ProjectConfig`

This is a plain function, not a trait. Web platforms can resolve config
separately and still produce the same `ProjectConfig`.

### Initial config schema

The shipped config schema is intentionally narrow. `.agents/config.toml`
currently supports:
- `[agent]`
- `[agent.inference]`

Richer provider, tools, session, and MCP declarations remain future work and
should be tracked under their own changes rather than this one.

### AGENTS.md integration (runtime)

The shipped AGENTS behavior is split across two runtime helpers:
- `<project-root>/.agents/AGENTS.md` is used as the root system prompt fallback
  when config does not already declare `agent.system_prompt`
- nested `AGENTS.md` files between the project root and current working
  directory are collected dynamically as runtime context

This remains a runtime concern rather than part of config parsing itself.

### Layered filesystem config

The current filesystem resolution model is layered:

1. built-in defaults
2. global `~/.brain/config.toml`
3. project `.agents/config.toml`
4. runtime root `.agents/AGENTS.md` fallback and nested `AGENTS.md` context where applicable

The SDK remains project-first, but it does support a global base layer.

## Impact

- **Modified specs**: `brain-types`, `brain-core`
- **New module/crate**: `brain-config` (filesystem config resolution utility)
- **Modifies**: `brain-types` (`ProjectConfig` struct, `Project` struct)
- **Modifies**: `brain-core` project bootstrap and runtime config resolution
- **No breaking changes to Store consumers**: `dyn Store` still works,
  sub-traits (`ProjectStore`, `SessionStore`, `MessageStore`) available for
  granular bounds

## Change Dependencies

- **Requires**: none
- **Related**: future richer config schema work may be picked up by `add-mcp-client` or later provider/tool configuration changes

## Crate Dependencies

- `toml` crate for config parsing (used by `resolve_fs_config`)
