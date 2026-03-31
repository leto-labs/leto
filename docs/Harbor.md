# Harbor

## Overview

`Harbor` is the repo's current operational harness for external agent
benchmarks. This page is the production-facing guide for how we actually run
Harbor in this repo, what currently works, and how to validate the live
`agent-*` stack.

For benchmark-research framing and the full `terminal-bench@2.0` task matrix,
see [`research/benchmarks/Harbor.md`](research/benchmarks/Harbor.md).

## Supported Agent Surfaces

The repo currently uses six Harbor reference surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local direct `agent`
- repo-local `codex-acp`
- repo-local `agent-acp`

Current maturity:

- `codex` is the stable built-in baseline
- `mini-swe-agent` is a useful built-in comparison surface, but weaker on
  heavier environment-repair tasks
- `terminus-2` is a Harbor-native reference surface
- `agent` is the direct repo-local benchmark surface over `agent exec`
- `codex-acp` is the main repo-local ACP comparison path
- `agent-acp` is the repo-local ACP loop-validation path on the live stack

## Runner Interface

The canonical runner is:

```bash
./scripts/harbor-run.sh <agent> <dataset> [task-name]
```

The repo also exposes the same interface through `just`:

```bash
just harbor-run <agent> <dataset> [task-name]
```

Examples:

```bash
./scripts/harbor-run.sh agent hello-world@1.0
./scripts/harbor-run.sh codex hello-world@1.0
./scripts/harbor-run.sh terminus-2 terminal-bench-sample@2.0 chess-best-move
just harbor-run agent-acp terminal-bench-sample@2.0 regex-log
just harbor-run codex-acp terminal-bench-sample@2.0 regex-log
HARBOR_ENV=daytona HARBOR_MODEL=openai/gpt-5.3-codex just harbor-run terminus-2 terminal-bench-sample@2.0 configure-git-webserver
```

Behavior defaults:

- explicit `task-name` means a bounded single-task run by default
- dataset-only runs default to the full filtered dataset
- `HARBOR_N_ATTEMPTS` defaults to `1`
- `HARBOR_N_CONCURRENT` defaults to `1`
- `HARBOR_TIMEOUT_MULTIPLIER` defaults to `1.0`

Useful env vars:

- `HARBOR_MODEL`
- `HARBOR_ENV`
- `HARBOR_FORCE_BUILD`
- `HARBOR_DELETE`
- `HARBOR_JOB_NAME`
- `HARBOR_JOB_SUFFIX`
- `HARBOR_N_TASKS`
- `HARBOR_N_ATTEMPTS`
- `HARBOR_N_CONCURRENT`
- `HARBOR_TIMEOUT_MULTIPLIER`

Environment behavior:

- `HARBOR_ENV` maps directly to Harbor's `--env`
- `HARBOR_FORCE_BUILD` maps to `--force-build` / `--no-force-build`
- `HARBOR_DELETE` maps to `--delete` / `--no-delete`
- defaults stay `docker`, `true`, and `true`

Useful discovery command:

```bash
just harbor-datasets
```

## Install And Prerequisites

Install Harbor as an external tool:

```bash
uv tool install --with agent-client-protocol harbor==0.1.45
```

Or through the repo helper:

```bash
just harbor-install
```

Important local prerequisites:

- Docker must be available and healthy for `HARBOR_ENV=docker`
- `DAYTONA_API_KEY` is required for `HARBOR_ENV=daytona`
- built-in `codex` requires `OPENAI_API_KEY`
- `mini-swe-agent` accepts `MSWEA_API_KEY` or provider-specific keys such as
  `OPENAI_API_KEY`
- direct `agent` requires `OPENAI_API_KEY` or `HARBOR_API_KEY`
- `agent-acp` defaults to `auth_method=openai-api-key` and therefore also
  requires `OPENAI_API_KEY` or `HARBOR_API_KEY` unless the caller overrides
  the auth strategy
- `codex-acp` prefers host Codex auth from `~/.codex/auth.json`

Recommended first Daytona validation:

```bash
HARBOR_ENV=daytona \
HARBOR_FORCE_BUILD=false \
HARBOR_MODEL=openai/gpt-5.3-codex \
HARBOR_N_ATTEMPTS=1 \
HARBOR_N_CONCURRENT=1 \
HARBOR_JOB_NAME=terminus-2-daytona-terminal-bench-sample-2.0-configure-git-webserver-gpt-5.3-codex \
just harbor-run terminus-2 terminal-bench-sample@2.0 configure-git-webserver
```

That task currently resolves to a single-container Harbor environment, so
Daytona uses the direct sandbox strategy rather than DinD compose mode.

## Checkpoint Ladder

The repo standardizes on four bounded Harbor checkpoints:

1. `hello-world@1.0`
2. `terminal-bench-sample@2.0/regex-log`
3. `terminal-bench-sample@2.0/chess-best-move`
4. `terminal-bench-sample@2.0/sqlite-with-gcov`

These are the day-to-day checkpoints for:

- smoke testing Harbor integration
- comparing built-in, direct, and ACP surfaces
- validating loop changes before full-suite runs

## ACP Operational Notes

The repo-local ACP path is:

- `acp_client.py` for the Harbor-side ACP client
- `acp_base.py` for the shared ACP lifecycle
- `acp_codex.py` for Codex
- `agent_acp.py` for the repo-local `agent-acp` backend

For container-backed ACP runs:

1. the Harbor agent uploads host artifact copies into `/installed-agent/staged/...`
2. Harbor runs the backend-specific install script inside the task container
3. the backend is launched inside Harbor's Docker environment
4. the host-side ACP client connects to it over stdio

Important distinction:

- staged ACP backend files are not bind mounts
- they are separate in-container copies created via Harbor upload APIs

## Resource Model And Concurrency

Harbor task environments usually inherit Harbor's default machine limits:

- `1 CPU`
- `2G RAM`
- `10G disk`

Some tasks override those limits, especially heavier build or ML tasks. The
full task matrix in [`research/benchmarks/Harbor.md`](research/benchmarks/Harbor.md)
records those task-level limits.

Operational guidance:

- for leaderboard-like runs, keep task resource limits unchanged
- increase throughput first with `HARBOR_N_CONCURRENT`
- use checkpoint tasks before launching the full suite
- treat full `terminal-bench@2.0` runs as the expensive validation stage

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
- direct `agent` artifacts such as `run.json` and `events.jsonl`
- Rust-emitted ATIF `trajectory.json` for the repo-local direct surface

The Docker task environments are ephemeral. The Harbor artifacts are not.

## Validation Focus

Current Harbor validation should prioritize the live migrated surfaces:

- direct `agent` on `hello-world@1.0`
- `agent-acp` on `hello-world@1.0`
- bounded `terminal-bench-sample@2.0` probes for both repo-local surfaces
- `codex` and `codex-acp` as external comparison baselines

The repo treats full-suite Harbor runs as a later validation stage after these
checkpoint surfaces are stable.

```text
max iterations reached: 20
```

So the largest exception bucket currently looks like a robust-loop iteration
budget problem, not a Harbor transport problem.

### Practical improvement priorities

The next high-value loop improvements are:

1. relax or redesign the robust-loop max-iteration limit
2. improve long-running shell-step timeout handling and final validation
3. tighten completion discipline on harder verifier-driven tasks

For the deeper benchmark-to-loop analysis and the design implications behind
those priorities, see [`research/benchmarks/LoopDesign.md`](research/benchmarks/LoopDesign.md).
