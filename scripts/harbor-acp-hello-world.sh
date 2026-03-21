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

HARBOR_DATASET="${HARBOR_DATASET:-hello-world@1.0}"
HARBOR_TASK_NAME="${HARBOR_TASK_NAME:-}"
HARBOR_N_TASKS="${HARBOR_N_TASKS:-1}"
HARBOR_N_ATTEMPTS="${HARBOR_N_ATTEMPTS:-1}"
HARBOR_N_CONCURRENT="${HARBOR_N_CONCURRENT:-1}"
HARBOR_TIMEOUT_MULTIPLIER="${HARBOR_TIMEOUT_MULTIPLIER:-1.0}"
HARBOR_MODEL="${HARBOR_MODEL:-openai/gpt-5.4}"
HARBOR_BACKEND="${HARBOR_BACKEND:-codex-acp}"
HARBOR_PERMISSION_MODE="${HARBOR_PERMISSION_MODE:-allow_once}"
HARBOR_SESSION_CWD="${HARBOR_SESSION_CWD:-/app}"
HARBOR_AGENT_IMPORT_PATH="${HARBOR_AGENT_IMPORT_PATH:-tools.harbor.agents.acp:AcpAgent}"
HARBOR_BACKEND_ARGS="${HARBOR_BACKEND_ARGS:-}"
HARBOR_BACKEND_CWD="${HARBOR_BACKEND_CWD:-}"
HARBOR_JOB_SUFFIX="${HARBOR_JOB_SUFFIX:-$(date -u +%Y%m%dT%H%M%SZ)}"
HARBOR_JOB_NAME="${HARBOR_JOB_NAME:-acp-${HARBOR_BACKEND}-hello-world-${HARBOR_JOB_SUFFIX}}"

case "${HARBOR_BACKEND}" in
  codex-acp)
    command -v codex-acp >/dev/null 2>&1 || {
      echo "codex-acp is required for HARBOR_BACKEND=codex-acp." >&2
      exit 1
    }
    HARBOR_BACKEND_COMMAND="${HARBOR_BACKEND_COMMAND:-codex-acp}"
    ;;
  brain-acp)
    command -v cargo >/dev/null 2>&1 || {
      echo "cargo is required for HARBOR_BACKEND=brain-acp." >&2
      exit 1
    }
    HARBOR_BACKEND_COMMAND="${HARBOR_BACKEND_COMMAND:-cargo}"
    HARBOR_BACKEND_ARGS="${HARBOR_BACKEND_ARGS:-run -q --manifest-path ${ROOT_DIR}/Cargo.toml -p brain-acp --bin brain-acp}"
    HARBOR_BACKEND_CWD="${HARBOR_BACKEND_CWD:-${ROOT_DIR}}"
    ;;
  *)
    echo "Unsupported ACP backend '${HARBOR_BACKEND}'." >&2
    echo "Supported backends: codex-acp, brain-acp." >&2
    exit 1
    ;;
esac

mkdir -p "${JOBS_DIR}"

cmd=(
  harbor jobs start
  --jobs-dir "${JOBS_DIR}"
  --job-name "${HARBOR_JOB_NAME}"
  --agent-import-path "${HARBOR_AGENT_IMPORT_PATH}"
  --model "${HARBOR_MODEL}"
  --dataset "${HARBOR_DATASET}"
  --n-tasks "${HARBOR_N_TASKS}"
  --n-attempts "${HARBOR_N_ATTEMPTS}"
  --n-concurrent "${HARBOR_N_CONCURRENT}"
  --timeout-multiplier "${HARBOR_TIMEOUT_MULTIPLIER}"
  --env docker
  --force-build
  --delete
  --ak "backend_command=${HARBOR_BACKEND_COMMAND}"
  --ak "permission_mode=${HARBOR_PERMISSION_MODE}"
  --ak "session_cwd=${HARBOR_SESSION_CWD}"
)

if [[ -n "${HARBOR_TASK_NAME}" ]]; then
  cmd+=(--task-name "${HARBOR_TASK_NAME}")
fi

if [[ -n "${HARBOR_BACKEND_ARGS}" ]]; then
  cmd+=(--ak "backend_args=${HARBOR_BACKEND_ARGS}")
fi

if [[ -n "${HARBOR_BACKEND_CWD}" ]]; then
  cmd+=(--ak "backend_cwd=${HARBOR_BACKEND_CWD}")
fi

echo "Harbor job artifacts will be written under: ${JOBS_DIR}/${HARBOR_JOB_NAME}"

PYTHONPATH="${ROOT_DIR}${PYTHONPATH:+:${PYTHONPATH}}" "${cmd[@]}"
