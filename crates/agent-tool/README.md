# Agent Tool Ecosystem

This folder groups the standalone crates that define and implement the tool
layer for an agent runtime.

The goal of this layout is to keep the tool ecosystem mentally separate from
the rest of the workspace. Everything under this folder should either define
the shared tool contract or implement concrete tool families on top of that
contract.

## Architecture

The tool ecosystem is split into two layers:

- `tool/`
  - the shared SDK and runtime-facing boundary
  - owns the common tool call/result types, executor traits, and standard
    registration path
- `tool-*`
  - standalone tool-family crates
  - implement concrete semantic tools such as file, process, or web-oriented
    capabilities

This keeps the core tool protocol stable while allowing individual tool
families to evolve independently.

## Dependency Direction

The intended dependency flow is:

- tool-family crates depend on `tool/`
- runtimes or application surfaces depend on `tool/` plus whichever `tool-*`
  families they want to install
- `tool/` does not depend on any specific tool-family crate

That separation makes it easier to:

- compose only the tool families needed by a given host
- keep the shared tool contract lightweight
- evolve or extract tool families independently

## Folder Layout

- `tool/` — shared tool SDK and erased executor contract
- `tool-files/` — canonical file and workspace tools
- `tool-process/` — canonical process tools
- `tool-web/` — reserved boundary for web-oriented tools

Additional tool-family crates should follow the same pattern:

- depend on `tool/`
- own one coherent capability area
- avoid reaching across sibling families unless there is a clear shared
  contract reason
