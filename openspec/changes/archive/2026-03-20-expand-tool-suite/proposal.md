# Proposal: expand-tool-suite

## Why

brain has 7 built-in tools (echo, file_read, file_write, file_edit, shell,
glob_search, grep). This covers the basics for a coding agent, but the next
step should focus on the highest-value additions for coding workflows:

1. **apply_patch / multi-edit**: models often want to make multiple edits in
   one tool call. `file_edit` only does single replacements. OpenCode and
   Vercel AI SDK offer `apply_patch` (unified diff format) for batched edits.
   This is also more token-efficient.

2. **directory listing / tree**: `glob_search` finds files by pattern but
   there's no way to see the directory structure. A `list_dir` or `tree` tool
   is standard in every coding agent.

3. **Better grep via ripgrep**: brain's native grep uses `walkdir` + `regex`,
   which works but is significantly slower than `rg` for large codebases.
   Competitors (OpenCode, pi-mono, ZeroClaw) all shell out to `rg`. We should
   offer an `rg`-backed driver as an alternative (falling back to native if
   `rg` is not installed).

## What

### New tools

| Tool | Name | Parameters | Description |
|------|------|-----------|-------------|
| ApplyPatchTool | `apply_patch` | `patch: String` | Apply a unified diff to one or more files |
| ListDirectoryTool | `list_directory` | `path: String, depth: Option<u32>` | List directory contents as a tree |

### Ripgrep-backed grep driver

A `GrepDriverRipgrep` that shells out to `rg` when available, falling back
to the existing `GrepDriverNative`. The tool name remains `grep` — only the
driver changes.

### Driver pattern preserved

All new tools follow the existing driver trait pattern:
- Driver trait with typed parameters
- Generic tool adapter implementing `Tool`
- Native driver implementation (feature-gated under `native`)

## Change Dependencies

- **Requires**: none (independent, can start immediately)
- **Enhances**: `add-config-system` (future tool enable/disable config, but not blocked by it)

## Impact

- **Modifies**: `tool-system` spec (new tools and grep implementation behavior)
- **New tools in**: `brain-tools` crate
- **Dependencies**: `diffy` for unified diff parsing
- **No breaking changes**: new tools are additive; existing `native_tools()` unchanged
