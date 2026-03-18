#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG="$SCRIPT_DIR/repocache.json"
TARGET_NAME="${1:-}"
NO_UPDATE="${2:-}"

if [ ! -f "$CONFIG" ]; then
  echo "Error: $CONFIG not found"
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq is required. Install with: sudo apt install jq"
  exit 1
fi

CLONED_URLS=()

clone_or_update() {
  local name="$1" url="$2" tag="$3"

  # Extract org/repo from URL
  local org_repo
  org_repo=$(echo "$url" | sed -E 's#.*github\.com[:/]##; s#\.git$##')
  local dest="$SCRIPT_DIR/$org_repo"

  # Deduplication: skip if already handled this URL in this run
  for u in "${CLONED_URLS[@]+"${CLONED_URLS[@]}"}"; do
    if [ "$u" = "$url" ]; then
      echo "  [$name] Shared clone at $org_repo (already handled)"
      return 0
    fi
  done
  CLONED_URLS+=("$url")

  if [ -d "$dest/.git" ]; then
    if [ "$NO_UPDATE" = "--no-update" ]; then
      echo "  [$name] Exists, skipping (--no-update)"
      return 0
    fi
    echo "  [$name] Updating $org_repo (tag: $tag)..."
    cd "$dest"
    if git fetch --depth 1 origin "refs/tags/$tag:refs/tags/$tag" 2>/dev/null; then
      git checkout --detach "refs/tags/$tag"
    else
      git fetch --depth 1 origin "$tag" 2>/dev/null || git fetch --depth 1 origin
      git checkout --detach FETCH_HEAD 2>/dev/null || git checkout "$tag" 2>/dev/null || true
    fi
    cd "$SCRIPT_DIR"
  else
    echo "  [$name] Cloning $org_repo (tag: $tag)..."
    mkdir -p "$(dirname "$dest")"
    # Try HTTPS first, fall back to SSH
    if ! git clone --depth 1 --branch "$tag" "$url" "$dest" 2>/dev/null; then
      if ! git clone --depth 1 "$url" "$dest" 2>/dev/null; then
        local ssh_url="git@github.com:${org_repo}.git"
        echo "  [$name] HTTPS failed, trying SSH..."
        git clone --depth 1 --branch "$tag" "$ssh_url" "$dest" 2>/dev/null || \
          git clone --depth 1 "$ssh_url" "$dest"
      fi
    fi
  fi
}

count=$(jq 'length' "$CONFIG")
echo "repocache: $count resources configured"
echo ""

for i in $(seq 0 $((count - 1))); do
  name=$(jq -r ".[$i].name" "$CONFIG")
  url=$(jq -r ".[$i].url" "$CONFIG")
  tag=$(jq -r ".[$i].tag // \"main\"" "$CONFIG")
  enabled=$(jq -r ".[$i].enabled // false" "$CONFIG")

  if [ "$enabled" != "true" ]; then
    continue
  fi

  if [ -n "$TARGET_NAME" ] && [ "$TARGET_NAME" != "$name" ]; then
    continue
  fi

  clone_or_update "$name" "$url" "$tag"
done

echo ""
echo "Done."
