# Design: add-harbor-benchmark-baseline

## Overview

This change keeps the Harbor implementation architecture intact, but cleans up
how the repo presents and invokes it.

The stable architectural pieces remain:

- Harbor is installed externally
- built-in Harbor agents are used directly for built-in surfaces
- repo-local ACP agents are loaded with `--agent-import-path`
- `codex-acp` and `brain-acp` both use the shared Harbor ACP client and shared
  ACP base

The design change is at the repo boundary:

- one benchmark capability
- one benchmark ladder
- one dedicated Harbor runner script
- one thin `just harbor-run <agent> <dataset> [task-name]` wrapper

## Benchmark Surfaces

The repo now has five reference agent surfaces that matter operationally:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local `codex-acp`
- repo-local `brain-acp`

They are not equally mature, so the benchmark spec and docs must encode support
level rather than implying one flat matrix of equal readiness.

## Benchmark Ladder

The common benchmark progression should stay concrete and bounded:

1. `hello-world@1.0`
2. `terminal-bench-sample@2.0/regex-log`
3. `terminal-bench-sample@2.0/chess-best-move`
4. `terminal-bench-sample@2.0/sqlite-with-gcov`

Why this shape:

- `hello-world` remains the cheapest ACP smoke test
- `regex-log` is the first bounded real synthesis task
- `chess-best-move` has already been useful for surfacing workspace and harness behavior
- `sqlite-with-gcov` is the heavier terminal/build/environment-repair task

## Public Interface

The repo should stop exposing separate shell scripts for:

- built-in versus ACP
- Codex versus Brain
- hello-world versus terminal-bench

Those differences are real, but they are implementation details, not the
public interface.

The primary runner interface should be:

- `./scripts/harbor-run.sh <agent> <dataset> [task-name]`

This needs dataset-first inputs because the real Harbor API works in terms of
datasets and optional task names, not repo-defined benchmark aliases.

The repo can also expose:

- `just harbor-run <agent> <dataset> [task-name]`

as a thin wrapper so the actual dispatch logic is not embedded in `justfile`.

## Script-Driven Dispatch

The dedicated runner script should own the actual Harbor dispatch.

The script should accept arbitrary Harbor datasets and optional task names
directly, while the docs and spec still define a recommended bounded ladder for
routine comparison.

For each agent surface, the script maps:

- built-in `--agent` versus repo-local `--agent-import-path`
- auth checks
- ACP-specific kwargs and environment requirements
- built-in `terminus-2` as a normal Harbor-native agent surface

This keeps the public surface concise while preserving the mode-specific Harbor
details internally, while also keeping `justfile` small and readable.

## ACP Architecture

No ACP architecture change is intended in this cleanup.

The repo-local Harbor ACP layout remains:

- `acp_client.py` for the Harbor ACP client
- `acp_base.py` for the shared ACP lifecycle
- `acp_codex.py` for the Codex implementation
- `acp_brain.py` for the Brain implementation

Harbor's installed-agent code in repocache remains the reference for the setup
lifecycle only:

- `repocache/harbor-framework/harbor/src/harbor/agents/installed/base.py`
- `repocache/harbor-framework/harbor/src/harbor/agents/installed/codex.py`

The runtime remains custom ACP stdio, not `BaseInstalledAgent` inheritance.
