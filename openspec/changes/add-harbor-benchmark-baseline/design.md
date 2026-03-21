# Design: add-harbor-benchmark-baseline

## Overview

This change adds the operational baseline for Harbor without coupling the repo
to Harbor's source tree. Harbor is installed externally as a tool. The repo
keeps only the assets needed to make early benchmark work reproducible:

- documentation
- safe helper scripts
- one repo-local ACP bridge agent
- one local template task
- an OpenSpec capability that describes the workflow

The design keeps the earlier benchmark conclusions intact:

- `Terminal-Bench` remains the primary benchmark family we care about
- Harbor is the harness direction for that work
- we benchmark mature built-in agents first
- we add `brain` later through the common harness rather than a one-off runner

## Installation Model

Harbor is treated as an external CLI, not a repo dependency and not a
submodule. The repo standardizes on:

```bash
uv tool install --with agent-client-protocol harbor==0.1.45
```

Reasons:

- matches Harbor's intended usage model
- avoids checking third-party benchmark source into the repo
- keeps `repocache` research-only
- makes the first workflow close to what a contributor will really execute

## Safety And Cost Controls

Harbor does not expose a trustworthy benchmark-wide cost ceiling. The repo
therefore enforces safe early defaults in documentation and scripts rather than
depending on harness-level budgeting.

The built-in paid baseline workflow is intentionally narrow:

- exactly one task
- one attempt
- one concurrent trial
- explicit timeout multiplier
- one selected model

This keeps the first real run bounded to one agent working one task.

## Paid Step-1 Baseline

The repo provides a built-in-agent script for one Harbor dataset task and a
custom-agent script for one Harbor dataset task. The default first paid PoC
dataset is:

- `hello-world@1.0`

That keeps the first run focused on “does the agent execute correctly under
Harbor at all?” rather than immediately paying for a more meaningful benchmark
slice.

The next dataset after that should be:

- `terminal-bench-sample@2.0`

The script still permits overriding the dataset or task name, but it keeps the
bounded run shape fixed unless the caller changes it deliberately.

## Repo-Local ACP Bridge

The repo-local Harbor custom agent is a generic ACP client implemented in
Python. Harbor still sees it as an agent because of its `BaseAgent` interface,
but protocol-wise it is a client launching and speaking to an ACP backend.

Why this shape:

- Harbor already exposes `--agent-import-path`
- the official ACP Python SDK gives us a client-side subprocess boundary
- the bridge can be reused across multiple ACP backends
- validating the protocol seam with `codex-acp` first is lower risk than
  starting with `brain-acp`

The first validated backend is:

- `codex-acp`

The bridge maps ACP client-owned capabilities onto Harbor's environment:

- `fs/read_text_file`
- `fs/write_text_file`
- permission requests
- terminal lifecycle calls

## Future `brain-acp` Integration

This change documents, but does not fully complete, the Harbor adapter path for
`brain-acp`.

Two levels are explicitly planned:

### Level 1: Minimal Runnable ACP Bridge

- repo-local custom agent loaded by `--agent-import-path`
- validated first with `codex-acp`
- goal is execution correctness, not immediate metrics parity

### Level 2: Rich Adapter

- run `brain-acp` successfully through the same ACP bridge
- richer metrics population
- better trajectory handling and export
- closer parity with Harbor's stronger built-in agents like Codex

This split matters because Harbor's richer adapters are meaningfully deeper
than the thin legacy `Terminal-Bench` launchers, and because `brain-acp` does
not yet expose the same client-owned filesystem and terminal behavior that the
bridge can already exercise with `codex-acp`.

## Alternatives Considered

### Add Harbor as a submodule

Rejected for now:

- too much vendored third-party churn
- not needed for step 1
- conflicts with the repo's use of `repocache` for source research only

### Execute from `repocache`

Rejected:

- user requirement is that `repocache` remains documentation/research only

### Implement `brain-acp` first

Rejected:

- we need to understand Harbor's real execution and artifact model first
- `codex-acp` is a stronger ACP reference implementation for the bridge itself
