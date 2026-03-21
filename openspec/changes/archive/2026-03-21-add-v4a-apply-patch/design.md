# Design: V4A apply_patch

## Summary

Implement `apply_patch` as a V4A patch parser and executor directly in
`brain-tools`. The tool continues to take a single `patch` string parameter,
but that string now uses the Codex/OpenCode V4A patch envelope instead of plain
unified diff.

## Decisions

- V4A is the only supported `apply_patch` input format.
- Paths in V4A patches must be relative; absolute paths are rejected.
- `Add File`, `Delete File`, `Update File`, and optional `Move to` are
  supported.
- Update hunks are applied using context matching against file contents rather
  than line-number-based unified diff parsing.
- `*** End of File` is accepted syntactically for update hunks.
- The tool definition text should mirror Codex/OpenCode expectations closely
  enough that models know which patch shape to produce.
- `repocache` gets a tracked `attractor` resource entry; the cloned repo remains
  local and gitignored like the other resources.

## Tradeoffs

- Replacing unified diff support is a breaking contract change for this one
  tool, but it aligns the backend with the target agent ecosystem.
- A native parser keeps the workspace self-contained instead of adding an
  external git dependency on the Codex parser crate.
