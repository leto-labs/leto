# Tasks: expand-tool-suite

## Implementation Checklist

### apply_patch
- [x] Define `ApplyPatchDriver` trait with `apply_patch(patch: &str) -> Result<String>`
- [x] Implement unified diff parsing with `diffy`
- [x] Implement `ApplyPatchDriverNative`: parse diff, apply hunks to files
- [x] Implement `ApplyPatchTool<T>` adapter with `apply_patch` tool name
- [x] Handle file creation and deletion in the native driver
- [x] Write tests with sample unified diffs

### list_directory
- [x] Define `ListDirectoryDriver` trait with `list_directory(path: &str, depth: Option<u32>) -> Result<String>`
- [x] Implement `ListDirectoryDriverNative`: walk directory, format as tree
- [x] Implement `ListDirectoryTool<T>` adapter with `list_directory` tool name
- [x] Respect common ignore patterns (.git, node_modules, target)
- [x] Write tests for directory tree output

### ripgrep driver
- [x] Implement `GrepDriverRipgrep`: detect `rg` on PATH, shell out, parse output
- [x] Implement fallback: if `rg` not found, use `GrepDriverNative`
- [x] Add an auto-selecting grep driver to `native_tools()`
- [x] Write tests for the auto driver and native grep behavior
