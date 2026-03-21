# Tasks: add-harbor-benchmark-baseline

## Implementation Checklist

- [x] Add a new `brain-benchmarks` OpenSpec capability for the Harbor baseline workflow
- [x] Write a proposal and design for Harbor-first benchmark adoption
- [x] Add repo-local Harbor documentation under `docs/benchmarks/`
- [x] Add a repo-owned Harbor `hello-world` task as a template for future local Harbor tasks
- [x] Add a Harbor install helper script pinned to a known version
- [x] Add a bounded paid Harbor built-in hello-world baseline script
- [x] Add a repo-local Harbor custom ACP bridge agent via `--agent-import-path`
- [x] Add a bounded Harbor ACP hello-world helper script using `codex-acp`
- [x] Add `just` targets for Harbor installation, dataset listing, and bounded benchmark helpers
- [x] Run a built-in paid Harbor benchmark task with API-backed agent credentials
- [x] Run the repo-local Harbor ACP bridge successfully against `hello-world@1.0` with `codex-acp`
- [ ] Run `brain-acp` successfully through the Harbor ACP bridge
- [ ] Add richer `brain` metrics and trajectory integration
