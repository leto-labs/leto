## Design

The repository will use `lefthook` as an external hook manager rather than a
Cargo build-script-based hook installer.

The root `lefthook.yml` will define a single `pre-commit` command that formats
only the staged Rust files passed in by Lefthook.

That command will call a repo script which:

1. filters the incoming path list down to existing `*.rs` files
2. runs `rustfmt --edition 2024 <files...>`
3. relies on Lefthook `stage_fixed: true` to restage any formatting changes

## Tradeoffs

### Why `lefthook`

`lefthook` keeps hook configuration committed in the repo and does not require
tying Git hook installation to Cargo build side effects.

### Why auto-format staged files instead of checking the whole workspace

This repository intentionally avoids `cargo fmt --all` because it can touch
unrelated files. Formatting only the staged Rust files preserves that rule while
still giving a convenient pre-commit workflow.
