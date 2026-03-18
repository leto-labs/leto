#!/usr/bin/env bash

set -euo pipefail

# Install the repo-owned Nori prompt pack into the user's active Nori commands
# directory by symlink. Keeping the source prompts in-repo makes them easy to
# version, while symlinks let Nori load them from its expected global location.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
source_dir="${repo_root}/tools/nori-prompts"
nori_home="${NORI_HOME:-${HOME}/.nori/cli}"
target_dir="${nori_home}/commands"

if [[ ! -d "${source_dir}" ]]; then
  echo "Prompt source directory not found: ${source_dir}" >&2
  exit 1
fi

mkdir -p "${target_dir}"

installed=0
for prompt in "${source_dir}"/*.md; do
  if [[ ! -e "${prompt}" ]]; then
    continue
  fi

  name="$(basename "${prompt}")"
  target="${target_dir}/${name}"

  if [[ -e "${target}" && ! -L "${target}" ]]; then
    echo "Refusing to replace non-symlink target: ${target}" >&2
    exit 1
  fi

  ln -sfn "${prompt}" "${target}"
  echo "linked ${target} -> ${prompt}"
  installed=$((installed + 1))
done

if [[ "${installed}" -eq 0 ]]; then
  echo "No prompt files found in ${source_dir}" >&2
  exit 1
fi

echo "Installed ${installed} Nori prompt symlink(s) into ${target_dir}"
