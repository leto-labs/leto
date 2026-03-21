# Tasks: add-tui-transport

## Archival Cleanup

- [x] Confirm the current implementation does not include a TUI feature or TUI frontend in `brain-cli`
- [x] Confirm the product direction is ACP-first with terminal UX delegated to external ACP clients
- [x] Re-scope this change as superseded rather than planned implementation work
- [x] Preserve the proposal/design only as historical context for the abandoned in-repo TUI direction
- [x] Run `openspec validate add-tui-transport --strict`
- [x] Archive the change without promoting its TUI deltas into canonical specs

## Validation Notes

- `brain-cli` currently ships a simple runtime-backed CLI and an ACP compatibility alias, not a Ratatui frontend.
- `brain-acp` is the active generic backend direction.
- Nori research in `docs/acp/Nori.md` recommends keeping `brain-acp` backend-focused and using ACP-native clients as the main TUI surface.
