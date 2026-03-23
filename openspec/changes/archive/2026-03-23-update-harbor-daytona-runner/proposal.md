## Why

The repo's Harbor runner currently hardcodes `--env docker`, always forces
environment builds, and always requires local Docker. That blocks the simplest
Daytona validation flow even though Harbor already supports `daytona`
natively.

We need one common runner surface that stays aligned with Harbor's CLI instead
of introducing a parallel Docker-only wrapper.

## What Changes

- Add `HARBOR_ENV` support to `scripts/harbor-run.sh` and forward it directly
  to Harbor's `--env`.
- Add `HARBOR_FORCE_BUILD` and `HARBOR_DELETE` support so the wrapper mirrors
  Harbor's boolean CLI flags.
- Require local Docker only when `HARBOR_ENV=docker`.
- Keep the existing `just harbor-run <agent> <dataset> [task-name]` wrapper
  and document Daytona usage via env vars rather than a second recipe.
- Document a Daytona single-task Terminus run using
  `terminal-bench-sample@2.0/configure-git-webserver`.

## Impact

- Existing Docker benchmark workflows keep the same defaults.
- Contributors can run Harbor on Daytona through the same repo-local runner
  surface.
- The benchmark docs and spec stay aligned with Harbor's native CLI model.
