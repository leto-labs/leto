# Design: expand-tool-suite

## Decisions

- Add new tools only when the public tool contract changes.
- Keep existing tool names stable when only the implementation changes.
- Use `apply_patch` and `list_directory` as new public tools.
- Keep `grep` as the public tool name and add a ripgrep-backed driver behind it.

## Notes

- `apply_patch` uses unified diff parsing and application through `diffy`.
- `list_directory` is optimized for deterministic, LLM-readable output rather
  than exact shell parity.
- `native_tools()` now includes the two new tools and a grep auto-driver that
  prefers `rg` when available.
