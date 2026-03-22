#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: ./scripts/harbor-run.sh <agent> <dataset> [task-name]" >&2
  echo "Examples:" >&2
  echo "  ./scripts/harbor-run.sh codex hello-world@1.0" >&2
  echo "  ./scripts/harbor-run.sh codex terminal-bench-sample@2.0 regex-log" >&2
  echo "  ./scripts/harbor-run.sh terminus-2 terminal-bench-sample@2.0 chess-best-move" >&2
  echo "  ./scripts/harbor-run.sh codex-acp terminal-bench-sample@2.0 sqlite-with-gcov" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JOBS_DIR="${ROOT_DIR}/target/harbor/jobs"
AGENT="$1"
DATASET="$2"
TASK_NAME="${3:-}"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "${ROOT_DIR}/.env"
  set +a
fi

if ! command -v harbor >/dev/null 2>&1; then
  echo "Harbor is not installed. Run ./scripts/harbor-install.sh first." >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required for Harbor benchmark runs." >&2
  exit 1
fi

HARBOR_DATASET="${HARBOR_DATASET:-${DATASET}}"
HARBOR_TASK_NAME="${HARBOR_TASK_NAME:-${TASK_NAME}}"
if [[ -n "${HARBOR_N_TASKS:-}" ]]; then
  resolved_n_tasks="${HARBOR_N_TASKS}"
elif [[ -n "${HARBOR_TASK_NAME}" ]]; then
  resolved_n_tasks="1"
else
  resolved_n_tasks=""
fi
HARBOR_N_ATTEMPTS="${HARBOR_N_ATTEMPTS:-1}"
HARBOR_N_CONCURRENT="${HARBOR_N_CONCURRENT:-1}"
HARBOR_TIMEOUT_MULTIPLIER="${HARBOR_TIMEOUT_MULTIPLIER:-1.0}"
HARBOR_MODEL="${HARBOR_MODEL:-openai/gpt-5.4}"
HARBOR_JOB_SUFFIX="${HARBOR_JOB_SUFFIX:-$(date -u +%Y%m%dT%H%M%SZ)}"

dataset_slug="$(printf '%s' "${HARBOR_DATASET}" | tr '@/:' '---' | tr -cs '[:alnum:]._-' '-')"
task_slug=""
if [[ -n "${HARBOR_TASK_NAME}" ]]; then
  task_slug="-$(printf '%s' "${HARBOR_TASK_NAME}" | tr '@/:' '---' | tr -cs '[:alnum:]._-' '-')"
fi
HARBOR_JOB_NAME="${HARBOR_JOB_NAME:-${AGENT}-${dataset_slug}${task_slug}-${HARBOR_JOB_SUFFIX}}"

mkdir -p "${JOBS_DIR}"

cmd=(
  harbor jobs start
  --jobs-dir "${JOBS_DIR}"
  --job-name "${HARBOR_JOB_NAME}"
  --model "${HARBOR_MODEL}"
  --dataset "${HARBOR_DATASET}"
  --n-attempts "${HARBOR_N_ATTEMPTS}"
  --n-concurrent "${HARBOR_N_CONCURRENT}"
  --timeout-multiplier "${HARBOR_TIMEOUT_MULTIPLIER}"
  --env docker
  --force-build
  --delete
)

if [[ -n "${resolved_n_tasks}" ]]; then
  cmd+=(--n-tasks "${resolved_n_tasks}")
fi

if [[ -n "${HARBOR_TASK_NAME}" ]]; then
  cmd+=(--task-name "${HARBOR_TASK_NAME}")
fi

case "${AGENT}" in
  codex-acp)
    if ! command -v codex-acp >/dev/null 2>&1; then
      echo "codex-acp must be installed on the host so its local binary can be copied into the container." >&2
      exit 1
    fi

    HARBOR_PERMISSION_MODE="${HARBOR_PERMISSION_MODE:-allow_once}"
    HARBOR_SESSION_CWD="${HARBOR_SESSION_CWD:-/app}"
    HARBOR_BACKEND_ARGS="${HARBOR_BACKEND_ARGS:-}"
    HARBOR_AUTH_METHOD="${HARBOR_AUTH_METHOD:-}"
    HARBOR_HOST_CODEX_AUTH_PATH="${HARBOR_HOST_CODEX_AUTH_PATH:-$HOME/.codex/auth.json}"
    HARBOR_ENABLE_TERMINAL="${HARBOR_ENABLE_TERMINAL:-true}"
    HARBOR_AGENT_IMPORT_PATH="${HARBOR_AGENT_IMPORT_PATH:-tools.harbor.agents.acp_codex:AcpCodexAgent}"

    cmd+=(
      --agent-import-path "${HARBOR_AGENT_IMPORT_PATH}"
      --ak "permission_mode=${HARBOR_PERMISSION_MODE}"
      --ak "session_cwd=${HARBOR_SESSION_CWD}"
      --ak "enable_terminal=${HARBOR_ENABLE_TERMINAL}"
    )

    if [[ -n "${HARBOR_BACKEND_ARGS}" ]]; then
      cmd+=(--ak "backend_args=${HARBOR_BACKEND_ARGS}")
    fi

    if [[ -n "${HARBOR_AUTH_METHOD}" ]]; then
      cmd+=(--ak "auth_method=${HARBOR_AUTH_METHOD}")
    fi

    if [[ -f "${HARBOR_HOST_CODEX_AUTH_PATH}" ]]; then
      cmd+=(--ak "host_codex_auth_path=${HARBOR_HOST_CODEX_AUTH_PATH}")
    elif [[ "${HARBOR_AUTH_METHOD}" != "openai-api-key" ]]; then
      echo "No host Codex auth file found at ${HARBOR_HOST_CODEX_AUTH_PATH}." >&2
      echo "Set HARBOR_AUTH_METHOD=openai-api-key with OPENAI_API_KEY, or provide HARBOR_HOST_CODEX_AUTH_PATH." >&2
      exit 1
    fi
    ;;
  brain-acp)
    HARBOR_PERMISSION_MODE="${HARBOR_PERMISSION_MODE:-allow_once}"
    HARBOR_SESSION_CWD="${HARBOR_SESSION_CWD:-/app}"
    HARBOR_BACKEND_ARGS="${HARBOR_BACKEND_ARGS:-}"
    HARBOR_BRAIN_LOOP="${HARBOR_BRAIN_LOOP:-}"
    HARBOR_HOST_BRAIN_HOME_PATH="${HARBOR_HOST_BRAIN_HOME_PATH:-$HOME/.brain}"
    HARBOR_BACKEND_ARTIFACT_PATH="${HARBOR_BACKEND_ARTIFACT_PATH:-}"
    HARBOR_ENABLE_TERMINAL="${HARBOR_ENABLE_TERMINAL:-true}"
    HARBOR_AGENT_IMPORT_PATH="${HARBOR_AGENT_IMPORT_PATH:-tools.harbor.agents.acp_brain:AcpBrainAgent}"

    if [[ ! -d "${HARBOR_HOST_BRAIN_HOME_PATH}/credentials" ]]; then
      echo "brain-acp requires ${HARBOR_HOST_BRAIN_HOME_PATH}/credentials on the host." >&2
      exit 1
    fi

    cmd+=(
      --agent-import-path "${HARBOR_AGENT_IMPORT_PATH}"
      --ak "permission_mode=${HARBOR_PERMISSION_MODE}"
      --ak "session_cwd=${HARBOR_SESSION_CWD}"
      --ak "enable_terminal=${HARBOR_ENABLE_TERMINAL}"
      --ak "host_brain_home_path=${HARBOR_HOST_BRAIN_HOME_PATH}"
    )

    if [[ -n "${HARBOR_BACKEND_ARGS}" ]]; then
      cmd+=(--ak "backend_args=${HARBOR_BACKEND_ARGS}")
    fi

    if [[ -n "${HARBOR_BRAIN_LOOP}" ]]; then
      cmd+=(--ak "brain_loop=${HARBOR_BRAIN_LOOP}")
    fi

    if [[ -n "${HARBOR_BACKEND_ARTIFACT_PATH}" ]]; then
      cmd+=(--ak "backend_artifact_path=${HARBOR_BACKEND_ARTIFACT_PATH}")
    fi
    ;;
  *)
    # Harbor built-in agents use the native --agent path. The repo documents
    # several reference agents, but the runner stays flexible and forwards any
    # built-in Harbor agent name here.
    case "${AGENT}" in
      codex)
        if [[ -z "${OPENAI_API_KEY:-}" ]]; then
          echo "OPENAI_API_KEY must be set for built-in Harbor codex runs." >&2
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
    esac

    cmd+=(--agent "${AGENT}")
    ;;
esac

echo "Harbor job artifacts will be written under: ${JOBS_DIR}/${HARBOR_JOB_NAME}"
PYTHONPATH="${ROOT_DIR}${PYTHONPATH:+:${PYTHONPATH}}" "${cmd[@]}"
