#!/usr/bin/env bash
set -euo pipefail

declare -a rust_files=()

for path in "$@"; do
  if [[ "$path" == *.rs && -f "$path" ]]; then
    rust_files+=("$path")
  fi
done

if [[ ${#rust_files[@]} -eq 0 ]]; then
  exit 0
fi

rustfmt --edition 2024 "${rust_files[@]}"
