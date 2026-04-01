# Proposal: split-agent-tool-family-crates

## Why

The current tool layering is backwards for the modular runtime we want to
support.

- `agent-runtime` has been acting as the owner of shared tool boundary types
  such as `ToolExecutor`, `ToolCall`, and `ToolExecutionResult`
- the old `agent-tools` crate bundled unrelated file and process capabilities
  into one implementation crate
- the monolithic crate shape makes it harder to treat tool families as
  first-class products and complicates wasm/component-oriented evolution

This change realigns the tool stack with the same pattern already used by the
provider side: a small shared SDK crate plus independently composable
implementation crates.

## What Changes

- add `agent-tool` as the shared tool SDK and runtime-facing tool executor
  boundary
- move the first-party file/workspace/search tools into `agent-tool-files`
- move the first-party shell/process tools into `agent-tool-process`
- add `agent-tool-web` as the stable destination for future web-oriented
  canonical tools
- update `agent-runtime` to consume `agent-tool` instead of owning the tool
  boundary itself
- update `agent-core` native assembly to compose explicit tool families instead
  of depending on a monolithic `agent-tools` crate
- retire `agent-tools` as the primary architecture and remove the corresponding
  spec requirement

## Impact

- breaks crate import paths for callers that still refer to `agent-tools`
- makes tool-family dependencies explicit in `agent-core`, `agent-acp`, and
  future hosts
- keeps tool names and provider-visible schemas stable while changing crate
  ownership
