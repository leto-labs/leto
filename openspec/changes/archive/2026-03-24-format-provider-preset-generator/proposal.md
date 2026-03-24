## Why

`provider-preset-gen` currently emits valid Rust source, but not source that is
stable under `rustfmt`. That means a later `cargo fmt --all` rewrites the
checked-in generated preset files even when the underlying model catalog data
has not changed.

This creates avoidable diff churn and makes `--check` less useful as a signal
of real generator drift.

## What Changes

- Update `provider-preset-gen` to format generated Rust source before writing
  files.
- Make `provider-preset-gen --check` compare against the formatted output, not
  the raw pre-format generator output.
- Regenerate the checked-in `provider-openai` preset files so they match the
  formatted output.

## Impact

- Generated preset files become stable under `rustfmt`.
- `cargo fmt --all` should no longer dirty `provider-openai/src/presets/` when
  the generator output is otherwise current.
