#!/usr/bin/env bash

set -euo pipefail

if ! command -v uv >/dev/null 2>&1; then
  echo "uv is required to install Harbor." >&2
  exit 1
fi

HARBOR_VERSION="${HARBOR_VERSION:-0.1.45}"

exec uv tool install --force --with agent-client-protocol "harbor==${HARBOR_VERSION}"
