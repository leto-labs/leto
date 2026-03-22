# Harbor

## Overview

`Harbor` is the repo's current operational harness for external agent
benchmarks. This page is the production-facing guide for how we actually run
Harbor in this repo, what currently works, and what the latest runs tell us
about the `brain` loop.

For benchmark-research framing and the full `terminal-bench@2.0` task matrix,
see [docs/benchmarks/Harbor.md](/home/leovigna/Documents/projects/leovigna/mauser/docs/benchmarks/Harbor.md).

## Supported Agent Surfaces

The repo currently uses six Harbor reference surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local direct `brain`
- repo-local `codex-acp`
- repo-local `brain-acp`

Current maturity:

- `codex` is the stable built-in baseline
- `mini-swe-agent` is a useful built-in comparison surface, but weaker on
  heavier environment-repair tasks
- `terminus-2` is a Harbor-native reference surface
- `brain` is the direct native benchmark surface for iterating on `brain`
  loops without ACP in the middle
- `codex-acp` is the main repo-local ACP comparison path
- `brain-acp` is real and usable, but still the surface we are actively
  improving rather than the stable baseline

For the internal native `ATIF` architecture behind the direct `brain` surface,
see [docs/atif.md](/home/leovigna/Documents/projects/leovigna/mauser/docs/atif.md).

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
./scripts/harbor-run.sh codex hello-world@1.0
./scripts/harbor-run.sh brain hello-world@1.0
./scripts/harbor-run.sh terminus-2 terminal-bench-sample@2.0 chess-best-move
just harbor-run codex-acp terminal-bench-sample@2.0 regex-log
HARBOR_BRAIN_LOOP=robust just harbor-run brain-acp terminal-bench@2.0
```

Behavior defaults:

- explicit `task-name` means a bounded single-task run by default
- dataset-only runs default to the full filtered dataset
- `HARBOR_N_ATTEMPTS` defaults to `1`
- `HARBOR_N_CONCURRENT` defaults to `1`
- `HARBOR_TIMEOUT_MULTIPLIER` defaults to `1.0`

Useful env vars:

- `HARBOR_MODEL`
- `HARBOR_JOB_NAME`
- `HARBOR_JOB_SUFFIX`
- `HARBOR_N_TASKS`
- `HARBOR_N_ATTEMPTS`
- `HARBOR_N_CONCURRENT`
- `HARBOR_TIMEOUT_MULTIPLIER`

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

- Docker must be available and healthy
- built-in `codex` requires `OPENAI_API_KEY`
- direct Harbor `brain` uses `OPENAI_API_KEY` by default and does not depend
  on `~/.brain`
- direct Harbor `brain` defaults to the Responses API when the selected
  OpenAI-compatible provider preset declares support, and falls back to
  chat completions otherwise
- `mini-swe-agent` accepts `MSWEA_API_KEY` or provider-specific keys such as
  `OPENAI_API_KEY`
- `codex-acp` prefers host Codex auth from `~/.codex/auth.json`
- `brain-acp` expects `~/.brain/credentials` and optional `~/.brain/config.toml`

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
- `acp_brain.py` for Brain

For container-backed ACP runs:

1. the Harbor agent uploads host artifact copies into `/installed-agent/staged/...`
2. Harbor runs the backend-specific install script inside the task container
3. the backend is launched inside Harbor's Docker environment
4. the host-side ACP client connects to it over stdio

Important distinction:

- staged ACP backend files are not bind mounts
- they are separate in-container copies created via Harbor upload APIs

`brain-acp` now prefers the portable musl artifact:

```text
target/x86_64-unknown-linux-musl/release/brain-acp
```

Build it with:

```bash
just build-release
```

This requires a musl toolchain such as `musl-gcc`.

## Resource Model And Concurrency

Harbor task environments usually inherit Harbor's default machine limits:

- `1 CPU`
- `2G RAM`
- `10G disk`

Some tasks override those limits, especially heavier build or ML tasks. The
full task matrix in [docs/benchmarks/Harbor.md](/home/leovigna/Documents/projects/leovigna/mauser/docs/benchmarks/Harbor.md)
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
- raw direct `brain` artifacts such as `run.json` and `events.jsonl`
- Rust-emitted ATIF `trajectory.json` for the direct `brain` surface

The Docker task environments are ephemeral. The Harbor artifacts are not.

## Findings From Recent Runs

### `brain-acp` robust full `terminal-bench@2.0` baseline

The first full-suite `brain-acp` robust run is:

- [brain-acp-terminal-bench-2.0-k1-musl](/home/leovigna/Documents/projects/leovigna/mauser/target/harbor/jobs/brain-acp-terminal-bench-2.0-k1-musl)

Aggregate result:

- `24/89` passes
- `48/89` verifier failures
- `23/89` trials with exceptions
- mean reward `0.2697`

Representative passes included:

- `fix-git`
- `git-leak-recovery`
- `headless-terminal`
- `portfolio-optimization`
- `query-optimize`
- `sparql-university`

### What this run proved

- the musl packaging fix worked across the full mixed-image suite
- the remaining bottlenecks are loop behavior, request failures, and timeout
  handling rather than startup portability
- `brain-acp` is viable enough for full-suite measurement, but not yet close to
  the built-in Codex baseline

### Main failure classes

1. `RequestError` tasks that all collapsed into backend iteration-limit failure
   behavior
2. `AgentTimeoutError` tasks, especially long or heavy tasks
3. clean verifier failures where the task ran end to end but the produced
   solution was wrong

Representative timeout edge case:

- [build-pov-ray result](/home/leovigna/Documents/projects/leovigna/mauser/target/harbor/jobs/brain-acp-terminal-bench-2.0-k1-musl/build-pov-ray__rrcbuzd/result.json)

That task reached verifier reward `1.0` but still timed out at the Harbor
agent layer, which indicates the agent got stuck in a final shell step after
essentially solving the task.

Representative request-error pattern:

- [mteb-leaderboard backend-stderr](/home/leovigna/Documents/projects/leovigna/mauser/target/harbor/jobs/brain-acp-terminal-bench-2.0-k1-musl/mteb-leaderboard__UiCV5zk/agent/backend-stderr.txt)

The request-error trials all showed the same backend signature:

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
those priorities, see [docs/benchmarks/LoopDesign.md](/home/leovigna/Documents/projects/leovigna/mauser/docs/benchmarks/LoopDesign.md).
