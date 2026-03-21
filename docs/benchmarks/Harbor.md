# Harbor

## Overview

`Harbor` is the repo's operational benchmark harness for the current external
agent comparison work. The repo now uses five reference agent surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local `codex-acp`
- repo-local `brain-acp`

The Harbor runner logic lives in:

```bash
./scripts/harbor-run.sh <agent> <dataset> [task-name]
```

The repo also exposes the same interface through `just`:

```bash
just harbor-run <agent> <dataset> [task-name]
```

Where:

- `<agent>` is any Harbor built-in agent name or one of the repo-local ACP names
  `codex-acp` and `brain-acp`
- `<dataset>` is the Harbor dataset id such as `hello-world@1.0` or
  `terminal-bench-sample@2.0`
- `[task-name]` is optional and is used for single-task runs within a dataset

## Install Harbor

Harbor is installed as an external tool:

```bash
uv tool install --with agent-client-protocol harbor==0.1.45
```

Or through the repo helper:

```bash
just harbor-install
```

This repo does not vendor Harbor as a dependency or submodule. `repocache` is
research-only and should not be used as the runtime installation source.

## Benchmark Ladder

The repo standardizes on four bounded checkpoint targets for day-to-day
comparison:

1. `hello-world`
2. `regex-log`
3. `chess-best-move`
4. `sqlite-with-gcov`

Their Harbor mappings are:

- `hello-world` -> `hello-world@1.0`
- `regex-log` -> `terminal-bench-sample@2.0/regex-log`
- `chess-best-move` -> `terminal-bench-sample@2.0/chess-best-move`
- `sqlite-with-gcov` -> `terminal-bench-sample@2.0/sqlite-with-gcov`

These are documented checkpoints, not the only runs the script can execute.

All runs default to a bounded shape:

- `n_tasks=1`
- `n_attempts=1`
- `n_concurrent=1`
- `timeout_multiplier=1.0`

Override with env vars when needed:

- `HARBOR_MODEL`
- `HARBOR_JOB_NAME`
- `HARBOR_JOB_SUFFIX`
- `HARBOR_N_TASKS`
- `HARBOR_N_ATTEMPTS`
- `HARBOR_N_CONCURRENT`
- `HARBOR_TIMEOUT_MULTIPLIER`

## Running Benchmarks

Examples:

```bash
./scripts/harbor-run.sh codex hello-world@1.0
./scripts/harbor-run.sh terminus-2 terminal-bench-sample@2.0 chess-best-move
just harbor-run codex hello-world@1.0
just harbor-run mini-swe-agent terminal-bench-sample@2.0 regex-log
just harbor-run codex-acp terminal-bench-sample@2.0 chess-best-move
HARBOR_BRAIN_LOOP=robust just harbor-run brain-acp terminal-bench-sample@2.0 regex-log
just harbor-run codex terminal-bench-sample@2.0 sqlite-with-gcov
```

Useful discovery command:

```bash
just harbor-datasets
```

## Agent Notes

### Built-in `codex`

- uses Harbor's built-in `codex` integration
- requires `OPENAI_API_KEY`
- current stable built-in Harbor baseline

### Built-in `mini-swe-agent`

- uses Harbor's built-in `mini-swe-agent` integration
- accepts `MSWEA_API_KEY` or provider-specific auth such as `OPENAI_API_KEY`
- supported as a benchmark surface, but currently weaker on heavier environment-repair tasks like `sqlite-with-gcov`

### Built-in `terminus-2`

- uses Harbor's built-in `terminus-2` integration
- is registered as a native Harbor agent in repocache:
  [name.py](/home/leovigna/Documents/projects/leovigna/mauser/repocache/harbor-framework/harbor/src/harbor/models/agent/name.py),
  [factory.py](/home/leovigna/Documents/projects/leovigna/mauser/repocache/harbor-framework/harbor/src/harbor/agents/factory.py)
- serves as a fifth reference surface because it ships with Harbor itself

### Repo-local `codex-acp`

- Harbor agent import path:
  `tools.harbor.agents.acp_codex:AcpCodexAgent`
- uses the shared Harbor ACP client and shared `BaseAcpAgent`
- copies a local `codex-acp` binary into the Harbor task environment
- runs `codex-acp` inside the container over ACP stdio
- prefers host Codex auth from `~/.codex/auth.json`
- supports `HARBOR_AUTH_METHOD=openai-api-key` as fallback

### Repo-local `brain-acp`

- Harbor agent import path:
  `tools.harbor.agents.acp_brain:AcpBrainAgent`
- uses the shared Harbor ACP client and shared `BaseAcpAgent`
- copies a local `brain-acp` binary into the Harbor task environment
- stages `~/.brain/credentials` and optional `config.toml` into a fresh container `BRAIN_HOME`
- supports `HARBOR_BRAIN_LOOP=robust` for stronger bounded-task behavior

## ACP Architecture

The repo-local ACP architecture is:

- `acp_client.py` for the Harbor-side ACP client
- `acp_base.py` for the shared ACP lifecycle
- `acp_codex.py` for Codex
- `acp_brain.py` for Brain

The setup flow is modeled on Harbor's installed-agent lifecycle in repocache,
but the runtime is still custom ACP stdio rather than `BaseInstalledAgent`
inheritance.

For container-backed ACP runs:

1. the agent stages host artifact copies into `/installed-agent/staged/...`
2. Harbor runs the backend-specific install script in the container
3. the backend is launched inside the Harbor Docker environment
4. the host-side ACP client connects to it over stdio

Important distinction:

- staged ACP backend files are not bind mounts
- they are separate in-container copies created via Harbor upload APIs

## Artifacts

Harbor writes job and trial artifacts under:

```text
target/harbor/jobs/
```

This includes:

- job-level `config.json`
- job-level `result.json`
- per-trial `result.json`
- agent logs
- verifier logs
- trajectories when the adapter emits them

The Docker environment is ephemeral; the Harbor artifacts are not.

## Current Reality

The five reference surfaces are not equally mature:

- `codex` is the stable built-in Harbor baseline
- `mini-swe-agent` is available and useful as a second built-in comparison surface
- `terminus-2` is a built-in Harbor-native reference surface
- `codex-acp` is the main repo-local ACP comparison path
- `brain-acp` is real and usable, but still a bounded validation path rather
  than the first stable real comparison baseline
