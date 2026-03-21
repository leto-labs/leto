#!/usr/bin/env bash

set -euo pipefail

if ! command -v harbor >/dev/null 2>&1; then
  echo "Harbor is not installed. Run ./scripts/harbor-install.sh first." >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for Harbor benchmark runs." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JOBS_DIR="${ROOT_DIR}/target/harbor/jobs"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "${ROOT_DIR}/.env"
  set +a
fi

HARBOR_AGENT="${HARBOR_AGENT:-mini-swe-agent}"
HARBOR_MODEL="${HARBOR_MODEL:-}"
HARBOR_DATASET="${HARBOR_DATASET:-hello-world@1.0}"
HARBOR_TASK_NAME="${HARBOR_TASK_NAME:-}"
HARBOR_N_TASKS="${HARBOR_N_TASKS:-1}"
HARBOR_N_ATTEMPTS="${HARBOR_N_ATTEMPTS:-1}"
HARBOR_N_CONCURRENT="${HARBOR_N_CONCURRENT:-1}"
HARBOR_TIMEOUT_MULTIPLIER="${HARBOR_TIMEOUT_MULTIPLIER:-1.0}"
HARBOR_JOB_NAME="${HARBOR_JOB_NAME:-${HARBOR_AGENT}-hello-world}"

if [[ -z "${HARBOR_MODEL}" ]]; then
  echo "HARBOR_MODEL is required for Harbor built-in baseline runs." >&2
  exit 1
fi

case "${HARBOR_AGENT}" in
  codex)
    if [[ -z "${OPENAI_API_KEY:-}" ]]; then
      echo "OPENAI_API_KEY must be set for codex runs." >&2
      exit 1
    fi
    ;;
  mini-swe-agent)
    if [[ -z "${MSWEA_API_KEY:-}" ]]; then
      provider="${HARBOR_MODEL%%/*}"
      case "${provider}" in
        anthropic)
          [[ -n "${ANTHROPIC_API_KEY:-}" ]] || {
            echo "Set ANTHROPIC_API_KEY or MSWEA_API_KEY for mini-swe-agent." >&2
            exit 1
          }
          ;;
        openai)
          [[ -n "${OPENAI_API_KEY:-}" ]] || {
            echo "Set OPENAI_API_KEY or MSWEA_API_KEY for mini-swe-agent." >&2
            exit 1
          }
          ;;
        *)
          echo "Set MSWEA_API_KEY for mini-swe-agent model provider '${provider}'." >&2
          exit 1
          ;;
      esac
    fi
    ;;
  *)
    echo "Unsupported built-in Harbor baseline agent '${HARBOR_AGENT}'." >&2
    echo "Use codex or mini-swe-agent, or extend scripts/harbor-builtin-hello-world.sh." >&2
    exit 1
    ;;
esac

mkdir -p "${JOBS_DIR}"

cmd=(
  harbor jobs start
  --jobs-dir "${JOBS_DIR}"
  --job-name "${HARBOR_JOB_NAME}"
  --agent "${HARBOR_AGENT}"
  --model "${HARBOR_MODEL}"
  --dataset "${HARBOR_DATASET}"
  --n-tasks "${HARBOR_N_TASKS}"
  --n-attempts "${HARBOR_N_ATTEMPTS}"
  --n-concurrent "${HARBOR_N_CONCURRENT}"
  --timeout-multiplier "${HARBOR_TIMEOUT_MULTIPLIER}"
  --env docker
  --force-build
  --delete
)

if [[ -n "${HARBOR_TASK_NAME}" ]]; then
  cmd+=(--task-name "${HARBOR_TASK_NAME}")
fi

echo "Harbor job artifacts will be written under: ${JOBS_DIR}/${HARBOR_JOB_NAME}"

"${cmd[@]}"
