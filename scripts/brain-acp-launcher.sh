#!/usr/bin/env bash
set -euo pipefail

# Stable external-client launcher for `brain acp`.
# This script exists so tools like `acpx` can spawn the local ACP server from
# arbitrary working directories without breaking Cargo workspace resolution.

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)

cd "${REPO_ROOT}"
exec cargo run -q -p brain-cli -- acp
