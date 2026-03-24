## 1. Hook configuration

- [x] Add committed `lefthook` configuration
- [x] Configure `pre-commit` to auto-format staged Rust files
- [x] Re-stage hook-fixed formatting changes automatically

## 2. Documentation

- [x] Document `lefthook install` setup in repo docs
- [x] Document that the hook formats only staged Rust files and does not run `cargo fmt --all`

## 3. Validation

- [x] Run `lefthook install`
- [x] Run `lefthook run pre-commit` against staged Rust files
- [x] Validate and archive the OpenSpec change
