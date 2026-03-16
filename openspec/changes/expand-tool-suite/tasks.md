# Tasks: expand-tool-suite

## Implementation Checklist

### apply_patch
- [ ] Define `ApplyPatchDriver` trait with `apply(patch: &str) -> Result<String>`
- [ ] Implement unified diff parser (or use `diffy` / `similar` crate)
- [ ] Implement `ApplyPatchDriverNative`: parse diff, apply hunks to files
- [ ] Implement `ApplyPatchTool<T>` adapter with `apply_patch` tool name
- [ ] Handle edge cases: file creation, file deletion, binary files, conflicts
- [ ] Write tests with sample unified diffs

### list_dir
- [ ] Define `ListDirDriver` trait with `list(path: &str, depth: Option<u32>) -> Result<String>`
- [ ] Implement `ListDirDriverNative`: walk directory, format as tree
- [ ] Implement `ListDirTool<T>` adapter with `list_dir` tool name
- [ ] Respect common ignore patterns (.git, node_modules, target)
- [ ] Write tests for directory tree output

### web_fetch
- [ ] Define `WebFetchDriver` trait with `fetch(url: &str) -> Result<String>`
- [ ] Implement `WebFetchDriverNative`: HTTP GET, return text content
- [ ] Implement `WebFetchTool<T>` adapter with `web_fetch` tool name
- [ ] Handle HTML → markdown conversion (optional, use `html2text` or similar)
- [ ] Add timeout and max response size limits
- [ ] Write tests with mock HTTP server

### ripgrep driver
- [ ] Implement `GrepDriverRipgrep`: detect `rg` on PATH, shell out, parse output
- [ ] Implement fallback: if `rg` not found, use `GrepDriverNative`
- [ ] Benchmark: ripgrep vs native grep on a medium codebase
- [ ] Write tests for ripgrep output parsing

### tool groups
- [ ] Define `coding_tools()` preset
- [ ] Define `readonly_tools()` preset
- [ ] Define `all_tools()` preset
- [ ] Keep `native_tools()` unchanged for backward compatibility
- [ ] Document preset contents
