#!/usr/bin/env bash
set -euo pipefail

# Stable external-client launcher for the explicit mock ACP binary.
# This keeps Cargo workspace resolution anchored at the repo root even when
# clients such as acpx spawn the backend from a temp working directory.

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)

cd "${REPO_ROOT}"
exec cargo run -q -p agent-acp --bin agent-acp-mock
