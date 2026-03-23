- [x] Update `scripts/harbor-run.sh` to support `HARBOR_ENV`,
  `HARBOR_FORCE_BUILD`, and `HARBOR_DELETE`
- [x] Make Docker a conditional prerequisite only for Docker-backed Harbor runs
- [x] Keep `just harbor-run` as the thin wrapper and clarify Daytona usage
- [x] Update Harbor benchmark docs with Docker and Daytona examples
- [x] Add a `brain-benchmarks` spec delta and validate it
- [x] Validate with `bash -n scripts/harbor-run.sh` and
  `openspec validate update-harbor-daytona-runner --strict`
