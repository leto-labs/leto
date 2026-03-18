#!/usr/bin/env bash
set -euo pipefail

# Opt-in external-client compatibility harness for the mock ACP server.
# This validates the real subprocess boundary with `acpx` instead of only
# relying on in-process Rust tests, and runs in a temp git repo so session and
# file flows do not depend on the current workspace state.

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)
LAUNCHER="${REPO_ROOT}/scripts/brain-acp-launcher.sh"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'missing required command: %s\n' "$1" >&2
    exit 1
  }
}

run_capture() {
  local label=$1
  shift

  printf '\n==> %s\n' "$label" >&2

  local output
  if ! output=$("$@" 2>&1); then
    printf 'command failed: %s\n' "$label" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi

  printf '%s\n' "$output" >&2
  printf '%s\n' "$output"
}

assert_contains() {
  local output=$1
  local needle=$2

  if [[ "$output" != *"$needle"* ]]; then
    printf 'expected output to contain: %s\n' "$needle" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

need_cmd acpx
need_cmd cargo
need_cmd git
need_cmd mktemp

export LC_ALL=C.UTF-8

WORKDIR=$(mktemp -d)
trap 'rm -rf "${WORKDIR}"' EXIT
git -C "${WORKDIR}" init -q

CODex_OUT=$(run_capture \
  "acpx codex exec hello world" \
  acpx --cwd "${WORKDIR}" codex exec "Reply exactly with: hello world")
assert_contains "${CODex_OUT}" "hello world"

HELLO_OUT=$(run_capture \
  "brain acp one-shot hello world" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" exec "Reply exactly with: hello world")
assert_contains "${HELLO_OUT}" "Mock brain-acp response: Reply exactly with: hello world"

PERMISSION_OUT=$(run_capture \
  "brain acp permission request" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" --approve-all exec "mock:request-permission")
assert_contains "${PERMISSION_OUT}" "Mock permission probe completed: allow-once."

READ_FILE="${WORKDIR}/mock-read.txt"
printf 'hello with unicode: café → résumé\n' > "${READ_FILE}"
READ_OUT=$(run_capture \
  "brain acp file read" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" exec "mock:read-file ${READ_FILE}")
assert_contains "${READ_OUT}" "Mock file read probe completed for ${READ_FILE}."

WRITE_FILE="${WORKDIR}/mock-write.txt"
WRITE_CONTENT="hello-write from acpx harness"
WRITE_OUT=$(run_capture \
  "brain acp file write" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" --approve-all exec "mock:write-file ${WRITE_FILE} ${WRITE_CONTENT}")
assert_contains "${WRITE_OUT}" "Mock file write probe completed for ${WRITE_FILE}."

TERMINAL_OUT=$(run_capture \
  "brain acp terminal flow" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" --approve-all exec "mock:terminal echo hi")
assert_contains "${TERMINAL_OUT}" "Mock terminal probe completed for \`echo hi\`"
assert_contains "${TERMINAL_OUT}" "exit=0"

TERMINAL_KILL_OUT=$(run_capture \
  "brain acp terminal kill flow" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" --approve-all exec "mock:terminal-kill sleep 5")
assert_contains "${TERMINAL_KILL_OUT}" "Mock terminal probe completed for \`sleep 5\`"
assert_contains "${TERMINAL_KILL_OUT}" "exit="

SESSION_NAME="compat"
run_capture \
  "brain acp sessions new" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" sessions new --name "${SESSION_NAME}" >/dev/null

STATUS_OUT=$(run_capture \
  "brain acp status" \
  acpx --cwd "${WORKDIR}" --agent "${LAUNCHER}" status -s "${SESSION_NAME}")
assert_contains "${STATUS_OUT}" "status:"

SHOW_OUT=$(run_capture \
  "brain acp sessions show" \
  acpx --format json --cwd "${WORKDIR}" --agent "${LAUNCHER}" sessions show "${SESSION_NAME}")
assert_contains "${SHOW_OUT}" "\"acpSessionId\":\"mock-session-1\""

LIST_OUT=$(run_capture \
  "brain acp sessions list" \
  acpx --format json --cwd "${WORKDIR}" --agent "${LAUNCHER}" sessions list)
assert_contains "${LIST_OUT}" "\"name\":\"${SESSION_NAME}\""

printf '\nacpx compatibility suite passed\n'
