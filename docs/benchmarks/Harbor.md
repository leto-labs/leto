# Harbor

## Overview

`Harbor` is the operational harness we should use for `brain`'s first external
benchmark baseline work. It is the current execution path for `Terminal-Bench`
2.0 and it also exposes a larger registry of datasets, built-in agent
integrations, and a custom-agent import path we can use later for `brain`.

This doc is not source research. It is the repo-owned operational workflow we
want contributors to follow when they need to install Harbor, run one tightly
bounded built-in benchmark baseline, and run the repo-local ACP bridge against
Harbor datasets.

## Step 1 Scope

The current implementation covers two Harbor paths:

1. install Harbor externally
2. inspect the CLI and dataset registry
3. run one bounded paid task with a built-in Harbor agent
4. run one bounded task through the repo-local ACP bridge with `codex-acp`

It does **not** yet run `brain-acp` successfully through Harbor. The current
custom agent is a generic ACP client bridge validated first with `codex-acp`.

## Install Harbor

Harbor should be installed as an external tool:

```bash
uv tool install --with agent-client-protocol harbor==0.1.45
```

Or through the repo helper:

```bash
just harbor-install
```

This repo does not vendor Harbor as a dependency or submodule. `repocache` is
research-only and should not be treated as the runtime installation source.

## Inspect The Live CLI

Useful discovery commands:

```bash
harbor --help
harbor datasets list
harbor jobs start --help
harbor trials start --help
```

The repo also exposes a convenience target:

```bash
just harbor-datasets
```

## Built-In Hello World Baseline

Run one real benchmark task with a built-in Harbor agent:

```bash
HARBOR_AGENT=mini-swe-agent \
HARBOR_MODEL=anthropic/claude-3-5-sonnet-20241022 \
ANTHROPIC_API_KEY=... \
just harbor-builtin-hello-world
```

Or with Codex:

```bash
HARBOR_AGENT=codex \
HARBOR_MODEL=openai/gpt-4o \
OPENAI_API_KEY=... \
just harbor-builtin-hello-world
```

Defaults used by the helper:

- dataset: `hello-world@1.0`
- tasks: `1`
- attempts: `1`
- concurrency: `1`
- timeout multiplier: `1.0`
- output dir: `target/harbor/jobs/`

## ACP Bridge Hello World

The repo-local custom Harbor agent is an ACP client implemented with the
official Python ACP SDK. It launches an ACP backend such as `codex-acp`, sends
the Harbor task instruction over ACP, and maps ACP filesystem / terminal calls
onto Harbor's task container.

Use this as the default custom-agent validation path:

```bash
OPENAI_API_KEY=... just harbor-acp-hello-world
```

Defaults used by the helper:

- dataset: `hello-world@1.0`
- backend: `codex-acp`
- Harbor custom agent: `tools.harbor.agents.acp:AcpAgent`
- model: `openai/gpt-5`
- tasks: `1`
- attempts: `1`
- concurrency: `1`
- timeout multiplier: `1.0`
- output dir: `target/harbor/jobs/`

Important notes:

- `codex-acp` still needs `OPENAI_API_KEY` or `CODEX_API_KEY`
- `gpt-4o` is not a safe default for `codex-acp` here; the validated default is
  `openai/gpt-5`
- the Harbor side is a custom ACP client; the spawned backend process is the
  ACP agent

Artifacts are kept on disk after the run. The helper passes `--delete` so the
Docker environment is torn down, but Harbor still keeps the job and trial
directories under `target/harbor/`.

Override points:

- `HARBOR_DATASET`
- `HARBOR_TASK_NAME`
- `HARBOR_N_TASKS`
- `HARBOR_N_ATTEMPTS`
- `HARBOR_N_CONCURRENT`
- `HARBOR_TIMEOUT_MULTIPLIER`
- `HARBOR_JOB_NAME`

## Cost Controls

Harbor exposes good task and concurrency controls, but it does not provide a
benchmark-wide dollar budget guard we should rely on. Cost control therefore
comes from how we invoke it.

Safe early defaults:

- exactly one task
- one attempt
- one concurrent trial
- explicit timeout multiplier
- one agent and one model only

That keeps the first paid run bounded to one agent working one task rather than
accidentally launching a larger sweep.

## What Harbor Saves

Harbor persists structured run data under the output directory you give it:

- job-level `config.json`
- job-level `result.json`
- per-trial `result.json`
- agent logs under each trial
- `agent/trajectory.json` when the agent adapter emits Harbor trajectory data

This means you do not lose the benchmark data when the container is deleted.
The Docker environment is ephemeral; the Harbor artifacts are not.

## Artifact Layout

Harbor organizes saved data in two layers:

1. job-level directory
2. one subdirectory per trial

For the `acp-codex-hello-world-gpt5` run we just executed, the layout looks
like:

```text
target/harbor/jobs/
  acp-codex-hello-world-gpt5/
    config.json
    result.json
    hello-world__zKajMWk/
      config.json
      result.json
      trial.log
      agent/
        assistant-response.txt
        backend-stderr.txt
        bridge.json
        session-updates.jsonl
        ...
      verifier/
        ctrf.json
        reward.txt
        test-stdout.txt
```

Interpretation:

- the job directory is the top-level benchmark run
- each subdirectory like `hello-world__zKajMWk` is one trial for one task
- `agent/` contains the agent-side logs and trajectories
- `verifier/` contains the benchmark/test output and reward files

For multi-task jobs, you should expect multiple trial subdirectories under the
same job directory, one for each task attempt Harbor executed.

Useful follow-up commands:

```bash
harbor view target/harbor/jobs
harbor jobs summarize target/harbor/jobs/<job-name>
```

`harbor jobs summarize` is optional and uses Claude Agent SDK for summaries, so
it is not part of the default baseline path.

## Recommended Datasets For Early Runs

Use the smallest meaningful progression:

1. `hello-world@1.0` through the ACP bridge for the first custom-agent PoC
2. `hello-world@1.0` through a built-in Harbor agent when comparing built-in vs custom paths
3. `terminal-bench-sample@2.0` with `HARBOR_N_TASKS=1` for the first non-trivial `Terminal-Bench` family run
4. only later consider broader `terminal-bench@2.0` or other coding datasets

## Built-In Agents To Start With

The first built-in agents worth testing are:

- `mini-swe-agent`
- `codex`

Those give us useful baseline coverage:

- a minimal agent scaffold
- a richer mature product integration

The paid smoke helper intentionally does not try to normalize every agent's
credential story. It supports the first likely paths:

- Codex with `OPENAI_API_KEY`
- mini-SWE-agent with `MSWEA_API_KEY` or a provider-specific key for common
  OpenAI/Anthropic model prefixes

## Repo-Owned Task Template And `oracle`

The repo still keeps a local Harbor task under
[`tools/harbor/tasks/hello-world`](../../tools/harbor/tasks/hello-world), but it
is no longer the primary benchmark workflow.

Use it as:

- a template for authoring custom Harbor tasks
- a reference for task layout, verifier shape, and `solution/solve.sh`

Harbor's built-in `oracle` agent is not an LLM agent. It runs the task's
reference solution from `solution/solve.sh` so benchmark authors can verify
that the task and verifier are sound.

## Future `brain-acp` Integration

The current ACP bridge is intentionally generic, but the first successful
backend is `codex-acp`, not `brain-acp`.

That matters because `brain-acp`'s real backend does not yet expose the same
client-owned filesystem and terminal behavior that makes the Harbor ACP bridge
useful for containerized tasks today. The next `brain-acp` step should build on
this ACP bridge rather than replacing it.
