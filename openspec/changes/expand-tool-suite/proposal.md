# Proposal: expand-tool-suite

## Why

brain has 7 built-in tools (echo, file_read, file_write, file_edit, shell,
glob_search, grep). This covers the basics for a coding agent, but production
agents need more:

1. **apply_patch / multi-edit**: models often want to make multiple edits in
   one tool call. `file_edit` only does single replacements. OpenCode and
   Vercel AI SDK offer `apply_patch` (unified diff format) for batched edits.
   This is also more token-efficient.

2. **directory listing / tree**: `glob_search` finds files by pattern but
   there's no way to see the directory structure. A `list_dir` or `tree` tool
   is standard in every coding agent.

3. **web fetch**: agents increasingly need to fetch URLs (documentation,
   APIs, etc.). A simple `web_fetch` tool that returns content as markdown
   is broadly useful.

4. **tool groups / presets**: instead of just `native_tools()` (all tools),
   provide named presets: `coding_tools()`, `readonly_tools()`,
   `shell_tools()`, etc. This lets clients configure which tools are available
   without manually constructing the list.

5. **Better grep via ripgrep**: brain's native grep uses `walkdir` + `regex`,
   which works but is significantly slower than `rg` for large codebases.
   Competitors (OpenCode, pi-mono, ZeroClaw) all shell out to `rg`. We should
   offer an `rg`-backed driver as an alternative (falling back to native if
   `rg` is not installed).

## What

### New tools

| Tool | Name | Parameters | Description |
|------|------|-----------|-------------|
| ApplyPatchTool | `apply_patch` | `patch: String` | Apply a unified diff to one or more files |
| ListDirTool | `list_dir` | `path: String, depth: Option<u32>` | List directory contents as a tree |
| WebFetchTool | `web_fetch` | `url: String` | Fetch URL content and return as text/markdown |

### Tool groups

```rust
fn coding_tools() -> Vec<Arc<dyn Tool>>    // file_read/write/edit, apply_patch, glob, grep, shell, list_dir
fn readonly_tools() -> Vec<Arc<dyn Tool>>  // file_read, glob, grep, list_dir
fn all_tools() -> Vec<Arc<dyn Tool>>       // everything including web_fetch, echo
```

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
- **Enhances**: `add-config-system` (tool enable/disable via config, but not blocked by it)

## Impact

- **Modifies**: `tool-system` spec (new tools, new presets)
- **New tools in**: `brain-tools` crate
- **Dependencies**: `reqwest` for web_fetch (already a dep via openai provider),
  `similar` or `diffy` crate for unified diff parsing
- **No breaking changes**: new tools are additive; existing `native_tools()` unchanged
