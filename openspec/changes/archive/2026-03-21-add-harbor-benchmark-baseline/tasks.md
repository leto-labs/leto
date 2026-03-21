# Tasks: add-harbor-benchmark-baseline

## Implementation Checklist

- [x] Add a new `brain-benchmarks` OpenSpec capability for the Harbor baseline workflow
- [x] Write a proposal and design for Harbor-first benchmark adoption
- [x] Add repo-local Harbor documentation under `docs/benchmarks/`
- [x] Add a repo-owned Harbor `hello-world` task as a template for future local Harbor tasks
- [x] Add a Harbor install helper script pinned to a known version
- [x] Add a repo-local Harbor custom ACP bridge agent via `--agent-import-path`
- [x] Add a backend-specific container-backed Harbor ACP Codex agent and install script
- [x] Add a backend-specific container-backed Harbor ACP Brain agent and install script
- [x] Validate built-in `codex` on bounded Harbor benchmark tasks
- [x] Validate built-in `mini-swe-agent` on bounded Harbor benchmark tasks
- [x] Add `terminus-2` as a built-in Harbor reference surface in docs/spec and runner support
- [x] Validate `codex-acp` on `hello-world@1.0` and selected `terminal-bench-sample@2.0` tasks
- [x] Validate `brain-acp` on `hello-world@1.0` and bounded real-task probes
- [x] Rewrite `brain-benchmarks` so it defines the five Harbor reference agent surfaces explicitly
- [x] Rewrite the Harbor docs around the four-step benchmark ladder
- [x] Remove the Harbor benchmark wrapper scripts
- [x] Move Harbor benchmark execution into `scripts/harbor-run.sh`
- [x] Keep `just harbor-run <agent> <dataset> [task-name]` as a thin wrapper over the dedicated runner
- [x] Remove the overly specific Harbor comparison helper script
- [x] Update Harbor/OpenSpec docs to match the cleaned runner-script interface
- [x] Run `openspec validate add-harbor-benchmark-baseline --strict`
- [x] Run static validation for the cleaned Harbor interface
- [x] Leave the worktree review-ready without creating a commit
