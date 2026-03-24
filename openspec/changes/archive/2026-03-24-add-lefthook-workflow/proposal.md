## Why

The repository currently relies on contributors remembering to format edited
Rust files before commit.

The repo also explicitly avoids `cargo fmt --all` as a routine fix-up command.
A hook-based workflow should format only the Rust files that are already staged
for commit.

## What Changes

- Add a committed `lefthook` configuration at the repository root.
- Auto-format staged Rust files in `pre-commit`.
- Document the expected `lefthook install` setup in repo docs.

## Impact

- Contributors get consistent local Rust formatting before commit.
- Formatting is scoped to the staged Rust files rather than the whole
  workspace.
