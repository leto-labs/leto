# Design: split-agent-tool-family-crates

## Decision Summary

We split the old `agent-tools` monolith into:

- `agent-tool` for shared tool contracts and the standard registry
- `agent-tool-files` for canonical file/workspace/search tools
- `agent-tool-process` for canonical process tools
- `agent-tool-web` as a stable family boundary for future web tools

## Rationale

### The shared tool boundary belongs below `agent-runtime`

`agent-runtime` should consume a tool executor boundary, not define it. Moving
the shared tool protocol into `agent-tool` fixes the dependency direction and
keeps tool authoring independent from PTY/process/runtime internals.

### Tool families are better seams than a single builtin tool crate

File/workspace/search tools and process tools evolve at different rates and
have different host capability needs. Splitting them into `agent-tool-*`
families keeps those concerns isolated while preserving one shared tool SDK.

### A thin web family boundary is valuable before concrete tools exist

`agent-tool-web` starts as a placeholder crate so future web fetch/browser
tools can land in a predictable family boundary without reopening the crate
layout question.

## Consequences

- `agent-core` now assembles its native registry by composing `agent-tool`,
  `agent-tool-files`, and `agent-tool-process`
- `agent-runtime` reexports shared tool types from `agent-tool` for transition
  convenience, but no longer owns those types
- any future wasm/component host integration should build on `agent-tool` plus
  the relevant `agent-tool-*` family crate rather than recreating a separate
  monolithic tool layer
