# Proposal: add-harbor-benchmark-baseline

## Why

`brain` needs a reproducible external benchmark path for comparing mature agent
products before we invest in a custom `brain` adapter. The earlier
`Terminal-Bench`-first plan is directionally right, but Harbor is now the
current harness direction for `Terminal-Bench 2.0` and the broader dataset
registry we care about.

We also need to control API spend carefully while learning the tool. Harbor
does not provide a benchmark-wide dollar budget guard that we can rely on, so
the repo needs an explicit low-risk workflow:

1. install Harbor externally
2. inspect available datasets and built-in agents
3. run exactly one paid task with hard bounds once a model and API key are set
4. run the repo-local ACP bridge against one Harbor dataset task

This lets us learn the real Harbor execution model before pushing on
`brain-acp`.

## What

### Step 1: Harbor baseline workflow

Add repo-owned documentation and helper scripts for a Harbor-first benchmark
workflow:

- install Harbor as an external tool with
  `uv tool install --with agent-client-protocol harbor==0.1.45`
- list Harbor datasets
- run a tightly bounded paid Harbor built-in hello-world baseline with agents such as
  `mini-swe-agent` or `codex`
- run a repo-local custom ACP bridge against `hello-world@1.0` using
  `codex-acp`

### Repo-local Harbor ACP bridge

Add a repo-local Harbor custom agent, loaded through `--agent-import-path`,
that uses the official Python ACP SDK as an ACP client and launches an ACP
backend process such as `codex-acp`.

### Repo-owned task template

Keep a tiny Harbor-compatible local task in the repo as a template for future
custom Harbor task authoring, not as the default benchmark workflow.

### Deferred `brain-acp` integration path

Document, but do not yet implement, the planned Harbor integration path for
`brain-acp`:

- Level 1: generic Harbor ACP bridge validated with `codex-acp`
- Level 2: run `brain-acp` through the same ACP bridge
- Level 2: richer metrics and trajectory-aware adapter

## Impact

- **New spec**: `brain-benchmarks`
- **New docs**: Harbor operational guidance under `docs/benchmarks/`
- **New scripts**: Harbor install and smoke-test helpers under `scripts/`
- **New custom agent**: repo-local Harbor ACP bridge under `tools/harbor/`
- **Template task**: repo-owned Harbor `hello-world` task under
  `tools/harbor/tasks/`
- **No current `brain-acp` parity**: the generic ACP bridge is implemented and
  validated first with `codex-acp`

## Non-Goals

- running broad benchmark sweeps
- fully integrating `brain-acp` into Harbor yet
- modifying Harbor upstream
- using `repocache` as the runtime source of truth
