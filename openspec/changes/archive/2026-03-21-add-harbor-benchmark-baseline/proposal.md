# Proposal: add-harbor-benchmark-baseline

## Why

`brain` now has a real Harbor benchmark surface, but the repo interface and
benchmark spec are still in a transitional state:

- the benchmark truth is still too Codex-first
- the reference agents are not presented cleanly in one place
- the repo has too many near-duplicate Harbor wrapper scripts

We need to wrap the current work into one coherent Harbor benchmark workflow
before reviewing and committing it.

## What

### Rewrite the benchmark capability around five reference agents

Keep the existing `brain-benchmarks` capability and make it the benchmark
source of truth for these five Harbor reference surfaces:

- built-in `codex`
- built-in `mini-swe-agent`
- built-in `terminus-2`
- repo-local `codex-acp`
- repo-local `brain-acp`

### Define a concrete benchmark ladder

Document one common bounded checkpoint ladder:

1. `hello-world@1.0`
2. `regex-log`
3. `chess-best-move`
4. `sqlite-with-gcov`

### Collapse the public Harbor interface into one runner script

Move the Harbor run assembly into a dedicated runner script:

- `./scripts/harbor-run.sh <agent> <dataset> [task-name]`

Keep `just` only as a thin wrapper around that script:

- `just harbor-run <agent> <dataset> [task-name]`

The command must encode both:

- the Harbor agent surface
- the Harbor dataset/task being run

## Impact

- **Modified spec**: `brain-benchmarks`
- **Modified spec**: `brain-acp` remains ACP-specific, not the benchmark truth
- **Modified docs**: Harbor benchmark docs under `docs/benchmarks/`
- **Removed scripts**: Harbor benchmark wrapper scripts under `scripts/`
- **Kept scripts**:
  - `scripts/harbor-install.sh`
  - `scripts/harbor-run.sh`
- **Modified interface**: `just harbor-run` becomes a thin wrapper over the dedicated Harbor runner

## Non-Goals

- committing the work in this pass
- broadening the benchmark ladder beyond the four identified checkpoints
- adding richer `brain-acp` metrics or trajectory export
- changing the underlying ACP agent implementations beyond what is needed for
  interface cleanup and doc/spec truthfulness
