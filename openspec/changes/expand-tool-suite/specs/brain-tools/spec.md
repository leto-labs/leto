# brain-tools Delta Spec

## ADDED Requirements

### Requirement: Apply Patch Tool
The system SHALL provide `ApplyPatchDriver` trait and `ApplyPatchDriverNative` implementation. `ApplyPatchTool<T>` SHALL expose tool name `apply_patch` with parameter `patch` (required string, unified diff format). The native driver SHALL parse the unified diff and apply hunks to the target files, creating or deleting files as specified by the diff.

#### Scenario: Apply multi-file patch
- **WHEN** `apply_patch` is called with a unified diff touching multiple files
- **THEN** it SHALL apply all hunks to the respective files
- **AND** return a summary of files modified

#### Scenario: Patch creates new file
- **WHEN** the diff contains a new file (--- /dev/null)
- **THEN** the tool SHALL create the file with the specified content

#### Scenario: Patch deletes file
- **WHEN** the diff specifies file deletion (+++ /dev/null)
- **THEN** the tool SHALL delete the file

#### Scenario: Patch conflicts
- **WHEN** a hunk cannot be applied because the context doesn't match
- **THEN** the tool SHALL return an error describing which hunk failed and why

### Requirement: List Directory Tool
The system SHALL provide `ListDirDriver` trait and `ListDirDriverNative` implementation. `ListDirTool<T>` SHALL expose tool name `list_dir` with parameters `path` (required string) and `depth` (optional integer, max recursion depth, default 3). The native driver SHALL list directory contents in a tree format, respecting common ignore patterns (.git, node_modules, target, __pycache__).

#### Scenario: List directory tree
- **WHEN** `list_dir` is called with `{"path": "."}`
- **THEN** it SHALL return a tree-formatted listing of the directory

#### Scenario: Depth limit
- **WHEN** `list_dir` is called with `{"path": ".", "depth": 1}`
- **THEN** it SHALL only list immediate children, not recurse deeper

#### Scenario: Ignore patterns
- **WHEN** the directory contains `.git/` or `node_modules/`
- **THEN** those directories SHALL be excluded from the listing

### Requirement: Web Fetch Tool
The system SHALL provide `WebFetchDriver` trait and `WebFetchDriverNative` implementation. `WebFetchTool<T>` SHALL expose tool name `web_fetch` with parameter `url` (required string). The native driver SHALL perform an HTTP GET request and return the response body as text. HTML responses SHOULD be converted to readable markdown/text.

#### Scenario: Fetch text content
- **WHEN** `web_fetch` is called with a URL returning plain text
- **THEN** it SHALL return the text content

#### Scenario: Fetch HTML content
- **WHEN** `web_fetch` is called with a URL returning HTML
- **THEN** it SHALL convert the HTML to readable text/markdown

#### Scenario: Fetch timeout
- **WHEN** the request exceeds a configurable timeout (default 30s)
- **THEN** it SHALL return an error indicating timeout

#### Scenario: Invalid URL
- **WHEN** `web_fetch` is called with an invalid or unreachable URL
- **THEN** it SHALL return an error describing the failure

### Requirement: Ripgrep Grep Driver
The system SHALL provide an alternative `GrepDriverRipgrep` implementation that shells out to `rg` (ripgrep) when available. If `rg` is not found on PATH, it SHALL fall back to `GrepDriverNative`. The tool name and parameters remain unchanged (`grep`).

#### Scenario: Ripgrep available
- **WHEN** `rg` is found on PATH
- **THEN** `GrepDriverRipgrep` SHALL use it for search operations

#### Scenario: Ripgrep not available
- **WHEN** `rg` is not found on PATH
- **THEN** `GrepDriverRipgrep` SHALL fall back to `GrepDriverNative` behavior

### Requirement: Tool Group Presets
The system SHALL provide named tool group functions:
- `coding_tools()` — file_read, file_write, file_edit, apply_patch, glob_search, grep, shell, list_dir
- `readonly_tools()` — file_read, glob_search, grep, list_dir
- `all_tools()` — all available tools including web_fetch and echo

The existing `native_tools()` function SHALL remain unchanged for backward compatibility.

#### Scenario: Coding tools preset
- **WHEN** `coding_tools()` is called
- **THEN** it SHALL return tools suitable for a coding agent (read, write, edit, search, shell)

#### Scenario: Readonly tools preset
- **WHEN** `readonly_tools()` is called
- **THEN** it SHALL return only tools that do not modify the filesystem or execute commands

#### Scenario: Native tools backward compat
- **WHEN** `native_tools()` is called
- **THEN** it SHALL return the same tools as before (echo + 6 native tools)


## ADDED Requirements

### Requirement: Apply Patch Tool
The system SHALL provide `ApplyPatchDriver` trait and `ApplyPatchDriverNative` implementation. `ApplyPatchTool<T>` SHALL expose tool name `apply_patch` with parameter `patch` (required string, unified diff format). The native driver SHALL parse the unified diff and apply hunks to the target files, creating or deleting files as specified by the diff.

#### Scenario: Apply multi-file patch
- **WHEN** `apply_patch` is called with a unified diff touching multiple files
- **THEN** it SHALL apply all hunks to the respective files
- **AND** return a summary of files modified

#### Scenario: Patch creates new file
- **WHEN** the diff contains a new file (--- /dev/null)
- **THEN** the tool SHALL create the file with the specified content

#### Scenario: Patch deletes file
- **WHEN** the diff specifies file deletion (+++ /dev/null)
- **THEN** the tool SHALL delete the file

#### Scenario: Patch conflicts
- **WHEN** a hunk cannot be applied because the context doesn't match
- **THEN** the tool SHALL return an error describing which hunk failed and why

### Requirement: List Directory Tool
The system SHALL provide `ListDirDriver` trait and `ListDirDriverNative` implementation. `ListDirTool<T>` SHALL expose tool name `list_dir` with parameters `path` (required string) and `depth` (optional integer, max recursion depth, default 3). The native driver SHALL list directory contents in a tree format, respecting common ignore patterns (.git, node_modules, target, __pycache__).

#### Scenario: List directory tree
- **WHEN** `list_dir` is called with `{"path": "."}`
- **THEN** it SHALL return a tree-formatted listing of the directory

#### Scenario: Depth limit
- **WHEN** `list_dir` is called with `{"path": ".", "depth": 1}`
- **THEN** it SHALL only list immediate children, not recurse deeper

#### Scenario: Ignore patterns
- **WHEN** the directory contains `.git/` or `node_modules/`
- **THEN** those directories SHALL be excluded from the listing

### Requirement: Web Fetch Tool
The system SHALL provide `WebFetchDriver` trait and `WebFetchDriverNative` implementation. `WebFetchTool<T>` SHALL expose tool name `web_fetch` with parameter `url` (required string). The native driver SHALL perform an HTTP GET request and return the response body as text. HTML responses SHOULD be converted to readable markdown/text.

#### Scenario: Fetch text content
- **WHEN** `web_fetch` is called with a URL returning plain text
- **THEN** it SHALL return the text content

#### Scenario: Fetch HTML content
- **WHEN** `web_fetch` is called with a URL returning HTML
- **THEN** it SHALL convert the HTML to readable text/markdown

#### Scenario: Fetch timeout
- **WHEN** the request exceeds a configurable timeout (default 30s)
- **THEN** it SHALL return an error indicating timeout

#### Scenario: Invalid URL
- **WHEN** `web_fetch` is called with an invalid or unreachable URL
- **THEN** it SHALL return an error describing the failure

### Requirement: Ripgrep Grep Driver
The system SHALL provide an alternative `GrepDriverRipgrep` implementation that shells out to `rg` (ripgrep) when available. If `rg` is not found on PATH, it SHALL fall back to `GrepDriverNative`. The tool name and parameters remain unchanged (`grep`).

#### Scenario: Ripgrep available
- **WHEN** `rg` is found on PATH
- **THEN** `GrepDriverRipgrep` SHALL use it for search operations

#### Scenario: Ripgrep not available
- **WHEN** `rg` is not found on PATH
- **THEN** `GrepDriverRipgrep` SHALL fall back to `GrepDriverNative` behavior

### Requirement: Tool Group Presets
The system SHALL provide named tool group functions:
- `coding_tools()` — file_read, file_write, file_edit, apply_patch, glob_search, grep, shell, list_dir
- `readonly_tools()` — file_read, glob_search, grep, list_dir
- `all_tools()` — all available tools including web_fetch and echo

The existing `native_tools()` function SHALL remain unchanged for backward compatibility.

#### Scenario: Coding tools preset
- **WHEN** `coding_tools()` is called
- **THEN** it SHALL return tools suitable for a coding agent (read, write, edit, search, shell)

#### Scenario: Readonly tools preset
- **WHEN** `readonly_tools()` is called
- **THEN** it SHALL return only tools that do not modify the filesystem or execute commands

#### Scenario: Native tools backward compat
- **WHEN** `native_tools()` is called
- **THEN** it SHALL return the same tools as before (echo + 6 native tools)
