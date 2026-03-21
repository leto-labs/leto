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

This lets us learn the real Harbor execution model and then validate both
`codex-acp` and `brain-acp` through the same Harbor ACP bridge.

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
- run `brain-acp` against the same Harbor registry task using the same
  repo-local ACP bridge

### Repo-local Harbor ACP bridge

Add a repo-local Harbor custom agent, loaded through `--agent-import-path`,
that uses the official Python ACP SDK as an ACP client and launches an ACP
backend process such as `codex-acp`.

### Repo-owned task template

Keep a tiny Harbor-compatible local task in the repo as a template for future
custom Harbor task authoring, not as the default benchmark workflow.

### `brain-acp` Harbor integration path

Implement the minimal working Harbor path for `brain-acp`:

- keep the Harbor ACP bridge generic and host-side
- launch `brain-acp` with the Nori-style local cargo command
- route the real backend's filesystem operations through ACP client
  capabilities so work lands in Harbor's Docker task workspace
- defer richer metrics and trajectory-aware adapter work

## Impact

- **New spec**: `brain-benchmarks`
- **Modified spec**: `brain-acp`
- **New docs**: Harbor operational guidance under `docs/benchmarks/`
- **New scripts**: Harbor install and smoke-test helpers under `scripts/`
- **New custom agent**: repo-local Harbor ACP bridge under `tools/harbor/`
- **Template task**: repo-owned Harbor `hello-world` task under
  `tools/harbor/tasks/`
- **Harbor ACP parity at hello-world scope**: the generic ACP bridge is now
  validated with both `codex-acp` and `brain-acp` on `hello-world@1.0`

## Non-Goals

- running broad benchmark sweeps
- richer `brain-acp` metrics / trajectory integration
- modifying Harbor upstream
- using `repocache` as the runtime source of truth
